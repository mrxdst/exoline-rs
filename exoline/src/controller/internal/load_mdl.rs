use super::{exo_file::ExoFile, util::split_once_and_trim_ascii};

pub struct LoadMdl {
    pub dpacs: Vec<LoadMdlItem>,
    pub tasks: Vec<LoadMdlItem>,
    pub texts: Vec<LoadMdlItem>,
}

pub struct LoadMdlItem {
    pub filename: String,
    pub load_number: Option<u8>,
    pub global: bool,
}

impl LoadMdl {
    pub fn parse(content: &str) -> Self {
        let exo_file = ExoFile::new(content);

        let mut load_mdl = LoadMdl {
            dpacs: Vec::new(),
            tasks: Vec::new(),
            texts: Vec::new(),
        };

        for section in exo_file.sections() {
            let list = match section.name.to_ascii_lowercase().as_str() {
                "dpac" => &mut load_mdl.dpacs,
                "task" => &mut load_mdl.tasks,
                "text" => &mut load_mdl.texts,
                _ => continue,
            };
            for item in section.items() {
                let mut split = item.line.split('/');
                let filename = match split.next() {
                    None => continue,
                    Some(filename) => filename.trim_ascii().to_string(),
                };
                if filename.is_empty() {
                    continue;
                }
                let mut load_number = None;
                let mut global = false;
                for flag in split {
                    let (flag, value) = split_once_and_trim_ascii(flag, '=');

                    match flag.to_ascii_uppercase().as_str() {
                        "MS" => global = true,
                        "LN" => load_number = value.map(|v| v.parse().ok()).flatten(),
                        _ => {}
                    }
                }

                list.push(LoadMdlItem {
                    filename,
                    load_number,
                    global,
                });
            }
        }

        load_mdl
    }
}
