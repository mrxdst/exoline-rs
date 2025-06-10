use std::{collections::HashMap, error::Error, io::Write, sync::Arc, time::Duration};

use chrono::DateTime;
use clap::Parser;
use comfy_table::{presets, CellAlignment, Table};
use exoline::{
    client::{EXOlineTCPClient, Variant},
    controller::{Controller, ControllerLoader, FileKind, LoadMode, VariableKind},
};
use rustyline::{completion::Completer, history::MemHistory, Editor, Helper, Highlighter, Hinter, Validator};
use tokio::{net::TcpStream, sync::Mutex, time::Instant};

use crate::{
    args::*,
    util::{format_variant, timeout_or_cancel},
};

use super::args::{Cli, ExportArgs, ReadArgs};

pub async fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    let host_port = match args.host {
        None => None,
        Some(host) => Some(format!("{}:{}", host, args.port)),
    };

    let loader = ControllerLoader::new_with_mode(Some(args.prod_dir), LoadMode::WithNamesAndComments);

    let controller = match loader.load_all(&args.controller_dir).await {
        Ok(controller) => controller,
        Err(err) => {
            println!("Error loading controller: {err}");
            println!("Attempting to load only system DPac's instead");
            loader.load_system().await
        }
    };

    println!("Loaded: {} files", controller.files().len());
    println!("address = {}:{}", controller.address.0, controller.address.1);
    println!();

    let mut client = ClientImpl::new(args.timeout, host_port, controller);

    client.command_loop().await?;

    Ok(())
}

struct ClientImpl {
    timeout: Duration,
    host_port: Option<String>,
    client: Arc<Mutex<Option<Arc<EXOlineTCPClient>>>>,
    controller: Arc<Controller>,
    last_table: Option<Table>,
    address: (u8, u8),
}

impl ClientImpl {
    pub fn new(timeout: Duration, host_port: Option<String>, controller: Controller) -> Self {
        Self {
            timeout,
            host_port,
            address: controller.address,
            client: Arc::new(Mutex::new(None)),
            controller: Arc::new(controller),
            last_table: None,
        }
    }

    async fn command_loop(&mut self) -> Result<(), Box<dyn Error>> {
        if self.host_port.is_some() {
            self.connect_if_needed().await?;
        }

        let config = rustyline::Config::builder().build();
        let helper = InteractiveHelper {
            controller: self.controller.clone(),
        };

        let mut rl = Editor::<InteractiveHelper, MemHistory>::with_history(config, MemHistory::new())?;
        rl.set_helper(Some(helper));
        let rl = Arc::new(Mutex::new(rl));

        loop {
            let _rl = rl.clone();
            let readline = tokio::spawn(async move { _rl.lock().await.readline("exoline-test> ") }).await?;

            match readline {
                Ok(line) => {
                    _ = rl.lock().await.add_history_entry(line.as_str());

                    println!();

                    let result = self.handle_command(line).await;

                    if let Ok(true) = result {
                        return Ok(());
                    }

                    if let Err(err) = result {
                        println!("{err}");
                    }

                    println!();
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, line: String) -> Result<bool, Box<dyn Error>> {
        let words = shellwords::split(&format!("exoline-test> {}", line))?;

        let cmd = Interactive::try_parse_from(words)?;

        let start = Instant::now();

        let result = match &cmd.command {
            InteractiveCommands::Info => self.info().await,
            InteractiveCommands::Read(args) => self.read(args).await,
            InteractiveCommands::Write(args) => self.write(args).await,
            InteractiveCommands::Dump(args) => self.dump(args).await,
            InteractiveCommands::Export(args) => self.export_csv(args).await,
            InteractiveCommands::Address => self.address().await,
            InteractiveCommands::Set(args) => match args.command {
                SetCommands::Host { ref host, port } => {
                    self.host_port = Some(format!("{}:{}", host, port));
                    return Ok(false);
                }
                SetCommands::Address { address } => {
                    self.address = address;
                    println!("address = {}:{}", address.0, address.1);
                    return Ok(false);
                }
                SetCommands::Timeout { timeout } => {
                    self.timeout = timeout;
                    println!("timeout = {}ms", timeout.as_millis());
                    return Ok(false);
                }
            },
            InteractiveCommands::Exit => return Ok(true),
        };

        let dur = Instant::now() - start;

        println!();
        println!("{}: {}ms", cmd.command, dur.as_millis());

        result.map(|_| false)
    }

    async fn info(&mut self) -> Result<(), Box<dyn Error>> {
        let client = self.connect_if_needed().await?;

        let mut table = Table::new();
        table.load_preset(presets::NOTHING);

        async fn get_value(
            this: &ClientImpl,
            client: &EXOlineTCPClient,
            load_number: u8,
            kind: VariableKind,
            offset: u32,
        ) -> Result<Variant, Box<dyn Error>> {
            Ok(timeout_or_cancel(
                this.timeout,
                client.read_variable_raw(this.address, FileKind::VPac, load_number, kind, offset),
            )
            .await??)
        }

        async fn get_partition_attribute(
            this: &ClientImpl,
            client: &EXOlineTCPClient,
            partition: u8,
            kind: VariableKind,
            id: u16,
        ) -> Result<Variant, Box<dyn Error>> {
            Ok(timeout_or_cancel(this.timeout, client.read_partition_attribute(this.address, partition, kind, id)).await??)
        }

        const Q_SYSTEM: u8 = 241;
        const Q_COM: u8 = 248;

        // QSystem.PLA, QSystem.ELA
        let pla = get_value(self, &client, Q_SYSTEM, VariableKind::Index, 0).await?.index().unwrap();
        let ela = get_value(self, &client, Q_SYSTEM, VariableKind::Index, 1).await?.index().unwrap();

        // QSystem.ModelText
        let model = get_value(self, &client, Q_SYSTEM, VariableKind::String, 100)
            .await?
            .string()
            .unwrap()
            .to_string();

        // QSystem.Ver_Major, QSystem.Ver_Minor, QSystem.Ver_Branch, QSystem.Ver_Number
        // let major = get_value(self, &client, Q_SYSTEM, VariableKind::Index, 17).await?.index().unwrap();
        // let minor = get_value(self, &client, Q_SYSTEM, VariableKind::Index, 16).await?.index().unwrap();
        // let branch = get_value(self, &client, Q_SYSTEM, VariableKind::Index, 40).await?.index().unwrap();
        // let number = get_value(self, &client, Q_SYSTEM, VariableKind::Index, 41).await?.index().unwrap();

        // QSystem.SerialNumberString
        let serial_number = get_value(self, &client, Q_SYSTEM, VariableKind::String, 60)
            .await?
            .string()
            .unwrap()
            .to_string();

        // QSystem.ControllerName, QSystem.ControllerProject
        let name = get_value(self, &client, Q_SYSTEM, VariableKind::String, 63)
            .await?
            .string()
            .unwrap()
            .to_string();
        let project = get_value(self, &client, Q_SYSTEM, VariableKind::String, 66)
            .await?
            .string()
            .unwrap()
            .to_string();

        // QCom.RunningIP
        let ip = get_value(self, &client, Q_COM, VariableKind::String, 162)
            .await?
            .string()
            .unwrap()
            .to_string();

        // QSystem.ActivePartition
        let active_partition = get_value(self, &client, Q_SYSTEM, VariableKind::Integer, 88).await?.integer().unwrap() as u8;

        // PartAttrHeader.LoadedBy, PartAttrHeader.LoadedDate
        let loaded_by = get_partition_attribute(self, &client, active_partition, VariableKind::String, 114)
            .await?
            .string()
            .unwrap()
            .to_string();
        let loaded_at = get_partition_attribute(self, &client, active_partition, VariableKind::Huge, 7)
            .await?
            .huge()
            .unwrap() as i64
            + 315532758;

        let loaded_at = DateTime::from_timestamp(loaded_at, 0).unwrap().with_timezone(&chrono::Local);

        table.add_row(["Project name", &project]);
        table.add_row(["Controller name", &name]);
        table.add_row(["Model", &model]);
        // table.add_row(["EXOreal version", &format!("{major}.{minor}.{branch}-{number:02}")]);
        table.add_row(["Serial number", &serial_number]);
        table.add_row(["EXOline address", &format!("{pla}:{ela}")]);
        table.add_row(["IP address", &ip]);
        table.add_row(["Loaded at", &loaded_at.to_string()]);
        table.add_row(["Loaded by", &loaded_by]);

        println!("{table}");
        self.last_table = Some(table);

        Ok(())
    }

    async fn address(&mut self) -> Result<(), Box<dyn Error>> {
        let client = self.connect_if_needed().await?;

        let (pla, ela) = timeout_or_cancel(self.timeout, client.read_exoline_address()).await??;

        self.address = (pla, ela);

        println!("address = {pla}:{ela}");

        Ok(())
    }

    async fn read(&mut self, args: &ReadArgs) -> Result<(), Box<dyn Error>> {
        let mut table = Table::new();
        table.load_preset(presets::NOTHING);
        table.add_row(["Variable", "Type", "Value", "Comment"]);
        table.column_mut(2).unwrap().set_cell_alignment(CellAlignment::Right);

        match self.controller.lookup_variable(&args.variable) {
            Some(variable) => {
                let client = self.connect_if_needed().await?;

                let value = timeout_or_cancel(self.timeout, client.read_variable(self.address, &variable)).await??;

                table.add_row([
                    variable.full_name().unwrap().as_str(),
                    &format!("{:?}", variable.kind()),
                    &format_variant(&value),
                    variable.comment().map(|s| s.to_string()).unwrap_or_default().as_str(),
                ]);
            }
            None => {
                let file = match self.controller.files().get(&args.variable) {
                    None => return Err("Unknown variable".into()),
                    Some(file) => file,
                };

                let client = self.connect_if_needed().await?;

                let values = timeout_or_cancel(self.timeout, client.read_dpac(self.address, &file)).await??;

                let mut variables = file.iter().collect::<Vec<_>>();
                variables.sort_by_key(|v| v.offset());

                for variable in variables {
                    let value_text = match values.get(&variable) {
                        None => "",
                        Some(value) => &format_variant(&value),
                    };
                    table.add_row([
                        variable.full_name().unwrap().as_str(),
                        &format!("{:?}", variable.kind()),
                        value_text,
                        variable.comment().map(|s| s.to_string()).unwrap_or_default().as_str(),
                    ]);
                }
            }
        }

        println!("{table}");
        self.last_table = Some(table);

        Ok(())
    }

    async fn write(&self, args: &WriteArgs) -> Result<(), Box<dyn Error>> {
        let variable = match self.controller.lookup_variable(&args.variable) {
            None => return Err("Unknown variable".into()),
            Some(variable) => variable,
        };

        let variant = match variable.kind() {
            VariableKind::Huge => Variant::Huge(args.value.parse()?),
            VariableKind::Index => Variant::Index(args.value.parse()?),
            VariableKind::Integer => Variant::Integer(args.value.parse()?),
            VariableKind::Logic => Variant::Logic(args.value.parse()?),
            VariableKind::Real => Variant::Real(args.value.parse()?),
            VariableKind::String => Variant::String(args.value.as_str().into()),
        };

        let client = self.connect_if_needed().await?;

        timeout_or_cancel(self.timeout, client.write_variable(self.address, &variable, &variant)).await??;

        Ok(())
    }

    async fn dump(&self, args: &DumpArgs) -> Result<(), Box<dyn Error>> {
        let values = match self.host_port {
            None => HashMap::new(),
            Some(_) => {
                let client = self.connect_if_needed().await?;
                let mut files = self.controller.dpacs().iter().collect::<Vec<_>>();
                files.sort_by(|a, b| a.name().cmp(b.name()));
                let mut values = HashMap::new();
                for file in files {
                    print!("Reading {}... ", file.name());
                    std::io::stdout().flush()?;
                    match timeout_or_cancel(self.timeout, client.read_dpac(self.address, &file)).await? {
                        Ok(data) => {
                            println!("OK");
                            values.extend(data)
                        }
                        Err(err) => println!("{err}"),
                    }
                }
                values
            }
        };

        let mut writer = csv::Writer::from_path(&args.filename)?;

        // writer.write_record(["Variable", "Type", "Segmented", "Offset", "Value", "Comment"])?;
        writer.write_record(["Variable", "Type", "Value", "Comment"])?;

        let mut files = self.controller.files().iter().collect::<Vec<_>>();
        files.sort_by(|a, b| a.load_number().cmp(&b.load_number()));

        let mut count = 0;

        for file in files.iter() {
            let mut variables = file.iter().collect::<Vec<_>>();
            variables.sort_by(|a, b| a.offset().cmp(&b.offset()));

            for variable in variables.iter() {
                count += 1;
                let value_text = match values.get(variable) {
                    None => "",
                    Some(value) => &format_variant(&value),
                };
                writer.write_record([
                    variable.full_name().unwrap().as_str(),
                    &format!("{:?}", variable.kind()),
                    // &format!("{:02X} {:02X}", variable.offset() / 60, variable.offset() % 60),
                    // &format!("{:04X}", variable.offset()),
                    value_text,
                    variable.comment().map(|s| s.to_string()).unwrap_or_default().as_str(),
                ])?;
            }
        }
        writer.flush()?;

        println!();
        if self.host_port.is_some() {
            println!("Read {} values", values.len());
        }
        println!("Dumped {count} variables");

        Ok(())
    }

    async fn export_csv(&self, args: &ExportArgs) -> Result<(), Box<dyn Error>> {
        let table = match &self.last_table {
            Some(table) => table,
            None => {
                println!("Nothing to export");
                return Ok(());
            }
        };

        let mut writer = csv::Writer::from_path(&args.filename)?;

        let header = table.header().unwrap().cell_iter().map(|c| c.content()).collect::<Vec<String>>();
        writer.write_record(header)?;

        for row in table.row_iter() {
            let record = row.cell_iter().map(|c| c.content()).collect::<Vec<String>>();
            writer.write_record(record)?;
        }
        writer.flush()?;

        println!("Exported");

        Ok(())
    }

    async fn connect_if_needed(&self) -> Result<Arc<EXOlineTCPClient>, Box<dyn Error>> {
        if let Some(client) = self.client.lock().await.as_ref() {
            return Ok(client.clone());
        }

        let addr = match &self.host_port {
            None => return Err("Missing host to connect to".into()),
            Some(addr) => addr,
        };

        print!("Connecting... ");
        std::io::stdout().flush()?;

        let stream = timeout_or_cancel(self.timeout, TcpStream::connect(addr)).await??;

        println!("Connected");
        println!();

        let (client, handle) = EXOlineTCPClient::new(stream);

        let client = Arc::new(client);

        _ = self.client.lock().await.insert(client.clone());

        let client_ = self.client.clone();

        tokio::spawn(async move {
            let result = handle.await.unwrap_or(Ok(()));
            _ = client_.lock().await.take();
            println!();
            println!();
            match result {
                Ok(_) => println!("Connection closed"),
                Err(err) => println!("{err}"),
            }
            println!();
        });

        Ok(client)
    }
}

#[derive(Helper, Hinter, Validator, Highlighter)]
struct InteractiveHelper {
    controller: Arc<Controller>,
}
const COMPLETIONS: [&str; 11] = [
    "info",
    "address",
    "read ",
    "write ",
    "set address ",
    "set host ",
    "set timeout ",
    "dump ",
    "export ",
    "help",
    "exit",
];

impl Completer for InteractiveHelper {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut matches = vec![];

        for cmd in COMPLETIONS {
            if cmd.starts_with(line) {
                matches.push(String::from(&cmd[pos..]));
            }
        }

        let prefix = if line.starts_with("read") {
            Some("read")
        } else if line.starts_with("write") {
            Some("write")
        } else {
            None
        };

        // only for break
        while let Some(prefix) = prefix {
            let line = line.to_lowercase();
            if !line.contains('.') {
                for file in self.controller.files().iter() {
                    let cmd = format!("{} {}", prefix, file.name());
                    if cmd.to_lowercase().starts_with(&line) {
                        matches.push(String::from(&cmd[pos..]));
                    }
                }
            } else {
                let (file, _) = line.split_once('.').unwrap();
                let parts = file.split_whitespace().collect::<Vec<_>>();
                if parts.len() != 2 {
                    break;
                }
                let filename = parts[1];
                let file = match self.controller.files().get(filename) {
                    None => break,
                    Some(file) => file,
                };
                for variable in file.iter() {
                    let cmd = format!("{} {}", prefix, variable.full_name().unwrap());
                    if cmd.to_lowercase().starts_with(&line) {
                        matches.push(String::from(&cmd[pos..]));
                    }
                }
            }

            break;
        }

        matches.sort();

        Ok((pos, matches))
    }
}
