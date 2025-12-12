use std::{fmt::Display, num::ParseIntError, path::PathBuf, time::Duration};

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Hostname or ip address. Leave empty to run without connecting.
    pub host: Option<String>,

    /// TCP port number
    #[arg(default_value = "26486")]
    pub port: u16,

    /// Network timeout in ms
    #[arg(short, long, default_value = "2000", value_parser = parse_duration)]
    pub timeout: Duration,

    /// Regin installation directory.
    /// Or the directory that contains SLib.
    #[arg(short, long, default_value = r"C:\Program Files\Regin")]
    pub prod_dir: PathBuf,

    /// Controller directory.
    #[arg(short, long, default_value = ".")]
    pub controller_dir: PathBuf,
}

#[derive(Parser, Debug)]
#[command()]
pub struct Interactive {
    #[command(subcommand)]
    pub command: InteractiveCommands,
}

#[derive(Subcommand, Debug)]
pub enum InteractiveCommands {
    /// Read info from device
    Info,

    /// Read values from device
    Read(ReadArgs),

    /// Write values to device
    Write(WriteArgs),

    /// Export all variables and values
    Dump(DumpArgs),

    /// Export the previously printed table
    Export(ExportArgs),

    /// Auto discover the EXOline address from device
    Address,

    /// Set configuration
    Set(SetArgs),

    /// Exit the program
    Exit,
}

impl Display for InteractiveCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractiveCommands::Info => write!(f, "Info"),
            InteractiveCommands::Read(_) => write!(f, "Read"),
            InteractiveCommands::Write(_) => write!(f, "Write"),
            InteractiveCommands::Dump(_) => write!(f, "Dump"),
            InteractiveCommands::Export(_) => write!(f, "Export"),
            InteractiveCommands::Address => write!(f, "Address"),
            InteractiveCommands::Set(_) => write!(f, "Set"),
            InteractiveCommands::Exit => write!(f, "Exit"),
        }
    }
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    /// Variable to read
    pub variable: String,
}

#[derive(Args, Debug)]
#[command(allow_negative_numbers = true)]
pub struct WriteArgs {
    /// Variable to write
    pub variable: String,

    /// Value to write
    pub value: String,
}

#[derive(Args, Debug)]
pub struct DumpArgs {
    /// The file to write to
    pub filename: PathBuf,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// The file to write to
    pub filename: PathBuf,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    #[command(subcommand)]
    pub command: SetCommands,
}

#[derive(Subcommand, Debug)]
pub enum SetCommands {
    /// Set the host and port
    Host {
        /// Hostname or ip address
        host: String,

        /// TCP port number
        #[arg(default_value = "26486")]
        port: u16,
    },

    /// Set the PLA:ELA
    Address {
        #[arg(value_parser = parse_pla_ela)]
        address: (u8, u8),
    },

    /// Set timeout
    Timeout {
        #[arg(value_parser = parse_duration)]
        timeout: Duration,
    },
}

fn parse_duration(input: &str) -> Result<Duration, ParseIntError> {
    let ms = input.parse()?;
    Ok(Duration::from_millis(ms))
}

fn parse_pla_ela(input: &str) -> Result<(u8, u8), Box<dyn std::error::Error + Send + Sync>> {
    match input.split_once(':') {
        Some((pla, ela)) => Ok((pla.parse()?, ela.parse()?)),
        None => Err("Invalid format".to_string().into()),
    }
}
