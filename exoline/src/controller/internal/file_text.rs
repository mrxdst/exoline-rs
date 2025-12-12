use super::super::{controller_loader::LoadMode, file::FileKind, variable::VariableKind};
use super::{
    exo_file::ExoFile,
    file_internal::{parse_variable_set_line, FileInternal, ParseFileError},
    util::split_once_and_trim_ascii,
    variable_map::VariableMap,
};

pub fn parse_text_file(content: &str, mode: LoadMode, hash: u64) -> Result<FileInternal, ParseFileError> {
    let exo_file = ExoFile::new(content);

    let mut name = None;
    let load_number = None;
    let mut variables = VariableMap::new(mode);

    let mut offset: u32 = 1;

    for section in exo_file.sections() {
        match section.name.to_ascii_lowercase().as_str() {
            "text" => {
                for item in section.items() {
                    let (key, value) = split_once_and_trim_ascii(item.line, '=');
                    if key.to_ascii_lowercase().as_str() == "name" {
                        name = value.map(|s| s.into())
                    }
                }
            }
            "strings" => {
                if !variables.is_empty() {
                    return Err(ParseFileError::InvalidSyntax("Strings section defined more than once".into()));
                }
                for item in section.items() {
                    let (kind, name, array_length) = parse_variable_set_line(item.line)?;

                    match array_length {
                        None => {
                            add_variable(&mut variables, &mut offset, kind, name, item.comment);
                        }
                        Some(len) => {
                            for i in 0..=len {
                                add_variable(&mut variables, &mut offset, kind, &format!("{}({})", name, i), item.comment);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(FileInternal {
        kind: FileKind::Text,
        name,
        load_number,
        variables,
        hash,
    })
}

fn add_variable(variables: &mut VariableMap, offset: &mut u32, kind: VariableKind, name: &str, comment: Option<&str>) {
    variables.insert(name.into(), kind, *offset, comment);
    *offset += 1;
}
