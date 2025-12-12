use std::{error::Error, fmt::Display};

use unicase::UniCase;

use super::super::{file::FileKind, variable::VariableKind};
use super::{util::split_once_and_trim_ascii, variable_map::VariableMap};

pub struct FileInternal {
    pub kind: FileKind,
    #[allow(unused)]
    pub name: Option<UniCase<String>>,
    pub load_number: Option<u8>,
    pub variables: VariableMap,
    pub hash: u64,
}

#[derive(Debug)]
pub enum ParseFileError {
    InvalidVariable(String),
    InvalidSyntax(String),
}

impl Display for ParseFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseFileError::InvalidVariable(err) => write!(f, "{err}"),
            ParseFileError::InvalidSyntax(err) => write!(f, "{err}"),
        }
    }
}

impl Error for ParseFileError {}

pub fn parse_variable_set_line(line: &str) -> Result<(VariableKind, &str, Option<u32>), ParseFileError> {
    let (key, _) = split_once_and_trim_ascii(line, '=');
    let (kind, rest) = key
        .split_at_checked(1)
        .ok_or_else(|| ParseFileError::InvalidVariable("Invalid variable syntax".into()))?;
    let Some(kind) = VariableKind::parse_from_char(kind.chars().next().unwrap()) else {
        return Err(ParseFileError::InvalidVariable("Invalid variable kind".into()));
    };
    let (rest, _) = split_once_and_trim_ascii(rest, ':'); // Discard string length
    let (name, array_length) = split_once_and_trim_ascii(rest, '['); // Array length

    if name.is_empty() {
        return Err(ParseFileError::InvalidVariable("Missing variable name".into()));
    }

    let array_length = match array_length {
        None => None,
        Some(s) => {
            let s = s
                .strip_suffix(']')
                .ok_or_else(|| ParseFileError::InvalidVariable("Invalid array syntax".into()))?;
            let len = s.parse().map_err(|_| ParseFileError::InvalidVariable("Invalid array syntax".into()))?;
            Some(len)
        }
    };

    Ok((kind, name, array_length))
}
