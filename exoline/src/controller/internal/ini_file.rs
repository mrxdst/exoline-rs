use std::sync::Arc;

use super::util::split_once_and_trim_ascii;

pub struct IniFile<'a> {
    lines: Arc<Vec<&'a str>>,
}

impl<'a> IniFile<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            lines: Arc::new(content.lines().collect()),
        }
    }

    pub fn sections(&self) -> IniFileSections<'a> {
        IniFileSections {
            lines: self.lines.clone(),
            line_nr: 0,
            initial: true,
        }
    }
}

pub struct IniFileSections<'a> {
    lines: Arc<Vec<&'a str>>,
    line_nr: usize,
    initial: bool,
}

impl<'a> Iterator for IniFileSections<'a> {
    type Item = IniFileSection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(line) = self.lines.get(self.line_nr) {
            let (line, _) = split_once_and_trim_ascii(line, ';');

            if line.is_empty() {
                self.line_nr += 1;
                continue;
            }

            let section_name = if line.starts_with('[') && line.ends_with(']') {
                Some(line[1..line.len() - 1].trim_ascii())
            } else {
                None
            };

            if section_name == None && self.initial {
                self.initial = false;
                return Some(IniFileSection {
                    name: section_name,
                    lines: self.lines.clone(),
                    line_nr: self.line_nr,
                });
            }

            self.initial = false;
            self.line_nr += 1;

            match section_name {
                Some(_) => {
                    return Some(IniFileSection {
                        name: section_name,
                        lines: self.lines.clone(),
                        line_nr: self.line_nr,
                    })
                }
                None => continue,
            }
        }

        return None;
    }
}

pub struct IniFileSection<'a> {
    pub name: Option<&'a str>,
    lines: Arc<Vec<&'a str>>,
    line_nr: usize,
}

impl<'a> IniFileSection<'a> {
    pub fn items(&self) -> IniFileItems<'a> {
        IniFileItems {
            lines: self.lines.clone(),
            line_nr: self.line_nr,
        }
    }
}

pub struct IniFileItems<'a> {
    lines: Arc<Vec<&'a str>>,
    line_nr: usize,
}

impl<'a> Iterator for IniFileItems<'a> {
    type Item = IniFileItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(line) = self.lines.get(self.line_nr) {
            let (line, comment) = split_once_and_trim_ascii(line, ';');

            if line.is_empty() {
                self.line_nr += 1;
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                return None;
            }

            self.line_nr += 1;

            let (key, value) = split_once_and_trim_ascii(line, '=');

            return Some(IniFileItem { key, value, comment });
        }

        return None;
    }
}

pub struct IniFileItem<'a> {
    pub key: &'a str,
    pub value: Option<&'a str>,
    #[allow(unused)]
    pub comment: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_it() {
        let text = "x = 99
y = 50
[s1]
a = 1
b = 2
[s2]
c = 3
d = 4
";
        let ini = IniFile::new(text);

        let mut str = String::new();

        for section in ini.sections() {
            if let Some(name) = section.name {
                str.push_str(&format!("[{name}]\n"));
            }
            for item in section.items() {
                str.push_str(&format!("{} = {}\n", item.key, item.value.unwrap()));
            }
        }

        assert_eq!(text, str);
    }
}
