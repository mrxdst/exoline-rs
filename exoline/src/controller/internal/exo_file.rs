use std::sync::Arc;

use super::util::split_once_and_trim_ascii;

pub struct ExoFile<'a> {
    lines: Arc<Vec<&'a str>>,
}

impl<'a> ExoFile<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            lines: Arc::new(content.lines().collect()),
        }
    }

    pub fn sections(&self) -> ExoFileSections<'a> {
        ExoFileSections {
            lines: self.lines.clone(),
            line_nr: 0,
        }
    }
}

pub struct ExoFileSections<'a> {
    lines: Arc<Vec<&'a str>>,
    line_nr: usize,
}

impl<'a> Iterator for ExoFileSections<'a> {
    type Item = ExoFileSection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(line) = self.lines.get(self.line_nr) {
            let (line, _) = split_once_and_trim_ascii(line, ';');

            if line.is_empty() {
                self.line_nr += 1;
                continue;
            }

            let section_name = line.strip_prefix("{").map(str::trim_ascii);

            self.line_nr += 1;

            match section_name {
                Some(name) => {
                    return Some(ExoFileSection {
                        name,
                        lines: self.lines.clone(),
                        line_nr: self.line_nr,
                    })
                }
                None => continue,
            }
        }

        None
    }
}

pub struct ExoFileSection<'a> {
    pub name: &'a str,
    lines: Arc<Vec<&'a str>>,
    line_nr: usize,
}

impl<'a> ExoFileSection<'a> {
    pub fn items(&self) -> ExoFileItems<'a> {
        ExoFileItems {
            lines: self.lines.clone(),
            line_nr: self.line_nr,
        }
    }
}

pub struct ExoFileItems<'a> {
    lines: Arc<Vec<&'a str>>,
    line_nr: usize,
}

impl<'a> Iterator for ExoFileItems<'a> {
    type Item = ExoFileItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(line) = self.lines.get(self.line_nr) {
            let (line, comment) = split_once_and_trim_ascii(line, ';');

            if line.is_empty() {
                self.line_nr += 1;
                continue;
            }

            if line.ends_with('}') {
                return None;
            }

            self.line_nr += 1;

            return Some(ExoFileItem { line, comment });
        }

        None
    }
}

pub struct ExoFileItem<'a> {
    pub line: &'a str,
    pub comment: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_it() {
        let text = "{ s1
a = 1
b = 2
}
{ s2
c = 3
d = 4
}
";
        let ini = ExoFile::new(text);

        let mut str = String::new();

        for section in ini.sections() {
            str.push_str(&format!("{{ {}\n", section.name));
            for item in section.items() {
                str.push_str(&format!("{}\n", item.line));
            }
            str.push_str("}\n");
        }

        assert_eq!(text, str);
    }
}
