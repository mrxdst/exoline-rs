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
                    for (i, item) in section.items().enumerate() {
                        match i {
                            0 => name = item.key.into(),
                            1 => {
                                let (pla_str, ela_str) = split_once_and_trim_ascii(item.key, '\t');
                                pla = pla_str.parse().unwrap_or(0);
                                ela = ela_str.and_then(|s| s.parse().ok()).unwrap_or(0);
                            }
                            2 => description = Some(item.key.into()),
                            _ => {}
                        }
                    }
                }
                Some(name) => if name.to_ascii_lowercase().as_str() == "module" {
                    for item in section.items() {
                        if item.key.to_ascii_lowercase().as_str() == "modulelibrary" {
                            module_library = item.value.map(|s| s.into())
                        }
                    }
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
