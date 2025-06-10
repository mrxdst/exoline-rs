use super::ini_file::IniFile;

pub struct TcpIpSettings {
    pub require_password: bool,
    pub system_password: Option<String>,
}

impl TcpIpSettings {
    pub fn parse(content: &str) -> Self {
        let ini_file = IniFile::new(content);

        let mut require_password = false;
        let mut system_password = None;

        for section in ini_file.sections() {
            if let Some(name) = section.name {
                match name.to_ascii_lowercase().as_str() {
                    "tcp/ip settings" => {
                        for item in section.items() {
                            match item.key.to_ascii_lowercase().as_str() {
                                "requirepassword" => {
                                    require_password = item.value.map(|s| s.to_ascii_lowercase()) == Some("yes".into());
                                }
                                "systempassword" => {
                                    system_password = item.value.map(|s| s.into());
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        TcpIpSettings {
            require_password,
            system_password,
        }
    }
}
