use std::borrow::Cow;

use super::super::{controller_loader::LoadMode, file::FileKind, variable::VariableKind};
use super::{
    exo_file::ExoFile,
    file_internal::{FileInternal, ParseFileError},
    util::split_once_and_trim_ascii,
    variable_map::VariableMap,
};

pub fn parse_bpac_file(exo_file: ExoFile, mode: LoadMode, hash: u64) -> Result<FileInternal, ParseFileError> {
    let mut name = None;
    let mut load_number = None;
    let mut variables = VariableMap::new(mode);

    let mut columns: Option<Vec<(VariableKind, Cow<str>)>> = None;
    let mut offset: u32 = 0;

    for section in exo_file.sections() {
        match section.name.to_ascii_lowercase().as_str() {
            "bpac" => {
                for item in section.items() {
                    let (key, value) = split_once_and_trim_ascii(item.line, '=');
                    match key.to_ascii_lowercase().as_str() {
                        "name" => name = value.map(|s| s.into()),
                        "ln" | "vln" => load_number = value.map(|s| s.parse().ok()).flatten(),
                        _ => {}
                    }
                }
            }
            "values" => {
                if !variables.is_empty() {
                    return Err(ParseFileError::InvalidSyntax("Values section defined more than once".into()));
                }
                let mut i: u32 = 0;
                let mut nr: u32 = 0;
                for item in section.items() {
                    let columns = match columns {
                        None => {
                            columns = Some(parse_bpac_column_headers(item.line)?);
                            continue;
                        }
                        Some(ref columns) => columns,
                    };
                    let _nr = item.line.split_once('#').map(|s| s.1.parse().ok()).flatten();
                    nr = u32::max(nr, _nr.unwrap_or(0));
                    while i < nr {
                        for (kind, name) in columns {
                            add_bpac_variable(&mut variables, &mut offset, *kind, &format!("Records({i}).{name}"), None);
                        }
                        i += 1;
                    }
                    for (kind, name) in columns {
                        add_bpac_variable(&mut variables, &mut offset, *kind, &format!("Records({i}).{name}"), item.comment);
                    }
                    i += 1;
                }
            }
            _ => {}
        }
    }

    Ok(FileInternal {
        kind: FileKind::BPac,
        name,
        load_number,
        variables,
        hash,
    })
}

fn parse_bpac_column_headers(line: &str) -> Result<Vec<(VariableKind, Cow<str>)>, ParseFileError> {
    let line = line[line.find(':').map(|i| i + 1).unwrap_or(0)..line.len() - 1].trim_ascii();
    let columns: Result<Vec<_>, ParseFileError> = line
        .split(':')
        .enumerate()
        .map(|(i, value)| -> Result<_, ParseFileError> {
            let value = value.trim_ascii();
            let (kind, name) = value
                .split_at_checked(1)
                .ok_or_else(|| ParseFileError::InvalidVariable("Invalid variable syntax".into()))?;
            // let kind = VariableKind::parse_from_char(kind.chars().nth(0).unwrap())
            //     .ok_or_else(|| ParseFileError::InvalidVariable("Invalid variable kind".into()))?;
            let Some(kind) = VariableKind::parse_from_char(kind.chars().nth(0).unwrap()) else {
                return Err(ParseFileError::InvalidVariable("Invalid variable kind".into()));
            };
            if kind == VariableKind::String {
                return Err(ParseFileError::InvalidVariable("BPac's can not contain strings".into()));
            }
            let name = match name.trim_ascii_start() {
                "" => format!("Record({i})").into(),
                name => name.into(),
            };
            Ok((kind, name))
        })
        .collect();
    Ok(columns?)
}

fn add_bpac_variable(variables: &mut VariableMap, offset: &mut u32, kind: VariableKind, name: &str, comment: Option<&str>) {
    let size = kind.offset_size_of_bpac_variable() as u32;
    variables.insert(name.into(), kind, *offset, comment);
    *offset += size;
}
