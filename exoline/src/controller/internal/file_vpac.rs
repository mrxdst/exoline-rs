use super::super::{controller_loader::LoadMode, file::FileKind, variable::VariableKind};
use super::{
    exo_file::ExoFile,
    file_internal::{parse_variable_set_line, FileInternal, ParseFileError},
    util::split_once_and_trim_ascii,
    variable_map::VariableMap,
};

pub fn parse_vpac_file(exo_file: ExoFile, mode: LoadMode, hash: u64) -> Result<FileInternal, ParseFileError> {
    let mut name = None;
    let mut pages: u32 = 1;
    let mut load_number = None;
    let mut align_with_segments = true;
    let mut variables = VariableMap::new(mode);

    let mut offset: u32 = 0;

    for section in exo_file.sections() {
        match section.name.to_ascii_lowercase().as_str() {
            "vpac" | "qpac" => {
                for item in section.items() {
                    let (key, value) = split_once_and_trim_ascii(item.line, '=');
                    match key.to_ascii_lowercase().as_str() {
                        "name" => name = value.map(|s| s.into()),
                        "ln" | "vln" => load_number = value.map(|s| s.parse().ok()).flatten(),
                        "pages" => pages = value.map(|s| s.parse().ok()).flatten().unwrap_or(1),
                        "alignwithsegments" => align_with_segments = value.map(|v| v.to_ascii_lowercase() == "yes").unwrap_or(true),
                        _ => {}
                    }
                }
            }
            "variables" => {
                if !variables.is_empty() {
                    return Err(ParseFileError::InvalidSyntax("Variables section defined more than once".into()));
                }
                for item in section.items() {
                    let (kind, name, array_length) = parse_variable_set_line(item.line)?;
                    add_vpac_variable(
                        &mut variables,
                        &mut offset,
                        kind,
                        name,
                        item.comment,
                        array_length,
                        pages,
                        align_with_segments,
                    );
                }
            }
            _ => {}
        }
    }

    Ok(FileInternal {
        kind: FileKind::VPac,
        name,
        load_number,
        variables,
        hash,
    })
}

fn add_vpac_variable(
    variables: &mut VariableMap,
    offset: &mut u32,
    kind: VariableKind,
    name: &str,
    comment: Option<&str>,
    array_length: Option<u32>,
    pages: u32,
    align_with_segments: bool,
) {
    let size = kind.offset_size_of_vpac_variable() as u32;

    // Not sure why string is special
    if align_with_segments && kind != VariableKind::String {
        let start_segment = *offset / 60;
        let end_segment = (*offset + size - 1) / 60;
        if start_segment != end_segment {
            *offset = (start_segment + 1) * 60;
        }
    }

    match array_length {
        None => {
            variables.insert(name.into(), kind, *offset, comment);

            if pages > 1 {
                for page in 1..pages {
                    let name = format!("Pages({}).{}", page, name).into();
                    variables.insert(name, kind, *offset + page * 60, comment);
                }
            }

            *offset += size;
        }
        Some(array_length) => {
            // align array
            if align_with_segments {
                let offset_in_segment = *offset % 60;
                if offset_in_segment + size + size * array_length > 60 {
                    *offset += (size - offset_in_segment % size) % 3;
                }
            }

            for i in 0..=array_length {
                let var_name = format!("{}({})", name, i).into();
                variables.insert(var_name, kind, *offset, comment);

                if pages > 1 {
                    for page in 1..pages {
                        let name = format!("Pages({}).{}({})", page, name, i).into();
                        variables.insert(name, kind, *offset + page * 60, comment);
                    }
                }

                *offset += size;
            }
        }
    }
}
