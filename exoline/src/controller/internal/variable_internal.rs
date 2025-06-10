use std::sync::Arc;

use super::super::variable::VariableKind;

pub trait VariableImpl {
    fn comment(&self) -> Option<Arc<String>>;

    fn kind(&self) -> VariableKind;

    fn offset(&self) -> u32;
}

#[derive(Clone, Debug)]
pub struct VerboseVariable {
    pub kind: VariableKind,
    pub comment: Option<Arc<String>>,
    pub offset: u32,
}

impl VariableImpl for VerboseVariable {
    fn kind(&self) -> VariableKind {
        self.kind
    }

    fn offset(&self) -> u32 {
        self.offset
    }

    fn comment(&self) -> Option<Arc<String>> {
        self.comment.clone()
    }
}

#[derive(Clone, Debug)]
pub struct StandardVariable(u32);

impl StandardVariable {
    pub fn new(kind: VariableKind, offset: u32) -> Self {
        let offset = offset & 0x00FF_FFFF;
        let kind = match kind {
            VariableKind::Huge => 0x00,
            VariableKind::Index => 0x01,
            VariableKind::Integer => 0x02,
            VariableKind::Logic => 0x03,
            VariableKind::Real => 0x04,
            VariableKind::String => 0x05,
        } << 24;
        Self(offset | kind)
    }
}

impl VariableImpl for StandardVariable {
    fn kind(&self) -> VariableKind {
        match self.0 >> 24 {
            0x00 => VariableKind::Huge,
            0x01 => VariableKind::Index,
            0x02 => VariableKind::Integer,
            0x03 => VariableKind::Logic,
            0x04 => VariableKind::Real,
            0x05 => VariableKind::String,
            _ => unreachable!(),
        }
    }

    fn offset(&self) -> u32 {
        self.0 & 0x00FF_FFFF
    }

    fn comment(&self) -> Option<Arc<String>> {
        None
    }
}

impl VariableKind {
    pub(crate) fn parse_from_char(value: char) -> Option<Self> {
        match value.to_ascii_uppercase() {
            'H' => Some(Self::Huge),
            'X' => Some(Self::Index),
            'I' => Some(Self::Integer),
            'L' => Some(Self::Logic),
            'R' => Some(Self::Real),
            '$' => Some(Self::String),
            _ => None,
        }
    }

    pub(crate) fn offset_size_of_vpac_variable(&self) -> u8 {
        match self {
            VariableKind::Huge => 3,
            VariableKind::Index => 1,
            VariableKind::Integer => 2,
            VariableKind::Logic => 1,
            VariableKind::Real => 3,
            VariableKind::String => 3,
        }
    }

    pub(crate) fn page_size_of_vpac_variable(&self) -> u8 {
        match self {
            VariableKind::Huge => 6,
            VariableKind::Index => 2,
            VariableKind::Integer => 4,
            VariableKind::Logic => 2,
            VariableKind::Real => 6,
            VariableKind::String => 6,
        }
    }

    pub(crate) fn offset_size_of_bpac_variable(&self) -> u8 {
        self.page_size_of_bpac_variable()
    }

    pub(crate) fn page_size_of_bpac_variable(&self) -> u8 {
        match self {
            VariableKind::Huge => 4,
            VariableKind::Index => 1,
            VariableKind::Integer => 2,
            VariableKind::Logic => 1,
            VariableKind::Real => 4,
            VariableKind::String => 1,
        }
    }
}
