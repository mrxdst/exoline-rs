use super::{ini_file::IniFile, util::split_once_and_trim_ascii};

pub struct ExistsMod {
    #[allow(unused)]
    pub name: String,
    pub pla: u8,
    pub ela: u8,
    #[allow(unused)]
    pub description: Option<String>,
    pub module_library: Option<String>,
}

impl ExistsMod {
    pub fn parse(content: &str) -> Self {
        let ini_file = IniFile::new(content);

        let mut name = String::default();
        let mut pla = 0;
        let mut ela = 0;
        let mut description = None;
        let mut module_library = None;

        for section in ini_file.sections() {
            match section.name {
                None => {
                    let mut i = 0;
                    for item in section.items() {
                        match i {
                            0 => name = item.key.into(),
                            1 => {
                                let (pla_str, ela_str) = split_once_and_trim_ascii(item.key, '\t');
                                pla = pla_str.parse().unwrap_or(0);
                                ela = ela_str.map(|s| s.parse().ok()).flatten().unwrap_or(0);
                            }
                            2 => description = Some(item.key.into()),
                            _ => {}
                        }
                        i += 1;
                    }
                }
                Some(name) => match name.to_ascii_lowercase().as_str() {
                    "module" => {
                        for item in section.items() {
                            match item.key.to_ascii_lowercase().as_str() {
                                "modulelibrary" => module_library = item.value.map(|s| s.into()),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
            }
        }

        Self {
            name,
            pla,
            ela,
            description,
            module_library,
        }
    }
}
