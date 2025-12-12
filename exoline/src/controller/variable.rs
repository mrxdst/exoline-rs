use std::{hash::Hash, sync::Arc};

use unicase::UniCase;

use super::{file::FileKind, internal::file_internal::FileInternal, File};

/// Holds required information that is needed to read data from a device.
#[derive(Clone)]
pub struct Variable {
    pub(crate) file: Arc<FileInternal>,
    pub(crate) file_name: Arc<UniCase<String>>,
    pub(crate) comment: Option<Arc<String>>,
    pub(crate) kind: VariableKind,
    pub(crate) offset: u32,
    pub(crate) name: Option<Arc<UniCase<String>>>,
    pub(crate) load_number: u8,
}

impl Variable {
    /// The kind of file the variable is defined in.
    pub fn file_kind(&self) -> FileKind {
        self.file.kind
    }

    /// The name of the file the variable is defied in.
    pub fn file_name(&self) -> &Arc<UniCase<String>> {
        &self.file_name
    }

    /// The file the variable is defied in.
    pub fn file(&self) -> File {
        File {
            file: self.file.clone(),
            file_key: self.file_name.clone(),
            load_number: self.load_number,
        }
    }

    /// The load number of the file the variable is defined in.
    pub fn load_number(&self) -> u8 {
        self.load_number
    }

    /// The datatype of the variable.
    pub fn kind(&self) -> VariableKind {
        self.kind
    }

    /// Only present when controller was loaded with [LoadMode::WithNames](crate::controller::LoadMode::WithNames).
    pub fn name(&self) -> Option<Arc<UniCase<String>>> {
        self.name.clone()
    }

    /// The fully qualified name for the variable in the controller.
    /// Only present when controller was loaded with [LoadMode::WithNames](crate::controller::LoadMode::WithNames).
    pub fn full_name(&self) -> Option<UniCase<String>> {
        self.name.as_ref().map(|name| UniCase::new(format!("{}.{}", self.file_name, name)))
    }

    /// Only present when controller was loaded with [LoadMode::WithNames](crate::controller::LoadMode::WithNamesAndComments) and the variable has a comment.
    pub fn comment(&self) -> Option<Arc<String>> {
        self.comment.clone()
    }

    /// Is `u24`. The most significant byte is discarded.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// The page in the file that the variable resides in.
    /// Only for DPac's. Other files just return 0.
    pub fn page(&self) -> u32 {
        match self.file.kind {
            FileKind::VPac => self.offset / 60,
            FileKind::BPac => self.offset / 120,
            FileKind::Task | FileKind::Text => 0,
        }
    }
}

impl PartialEq for Variable {
    /// Is considered equal if `self` and `other` would read the same variable from a device.
    fn eq(&self, other: &Self) -> bool {
        self.file.kind == other.file.kind && self.load_number == other.load_number && self.kind == other.kind && self.offset == other.offset
    }
}

impl Eq for Variable {}

impl Hash for Variable {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.file.kind {
            FileKind::BPac => 0u8.hash(state),
            FileKind::Task => 1u8.hash(state),
            FileKind::Text => 2u8.hash(state),
            FileKind::VPac => 3u8.hash(state),
        }
        self.load_number.hash(state);
        match self.kind {
            VariableKind::Huge => 0u8.hash(state),
            VariableKind::Index => 1u8.hash(state),
            VariableKind::Integer => 2u8.hash(state),
            VariableKind::Logic => 3u8.hash(state),
            VariableKind::Real => 4u8.hash(state),
            VariableKind::String => 5u8.hash(state),
        }
        self.offset.hash(state);
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum VariableKind {
    /// An [i32] value.
    Huge,
    /// An [u8] value.
    Index,
    /// An [i16] value.
    Integer,
    /// A [bool] value.
    Logic,
    /// A [f32] value.
    Real,
    /// A [String] value.
    String,
}
