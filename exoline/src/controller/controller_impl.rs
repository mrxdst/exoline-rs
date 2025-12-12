use std::{collections::HashMap, sync::Arc};

use unicase::UniCase;

use super::internal::file_internal::FileInternal;
use super::variable::Variable;
use super::FileSet;

pub(crate) type FileSetInternal = Arc<HashMap<Arc<UniCase<String>>, (u8, Arc<FileInternal>)>>;

/// Holds the controller configuration.
pub struct Controller {
    pub(crate) tasks: FileSetInternal,
    pub(crate) dpacs: FileSetInternal,
    pub(crate) texts: FileSetInternal,
    pub(crate) globals: FileSetInternal,

    /// The EXOline address. (PLA, ELA)
    pub address: (u8, u8),
    /// The device requires a password.
    pub require_password: bool,
    /// The password to use if required.
    pub system_password: Option<String>,
}

impl Controller {
    /// A collection of all Task files in the controller.
    pub fn tasks(&self) -> FileSet {
        FileSet {
            file_sets: vec![self.tasks.clone()],
        }
    }

    /// A collection of all DPac files in the controller.
    pub fn dpacs(&self) -> FileSet {
        FileSet {
            file_sets: vec![self.dpacs.clone()],
        }
    }

    /// A collection of all Text files in the controller.
    pub fn texts(&self) -> FileSet {
        FileSet {
            file_sets: vec![self.texts.clone()],
        }
    }

    /// A collection of all files in the controller.
    pub fn files(&self) -> FileSet {
        FileSet {
            file_sets: vec![self.dpacs.clone(), self.tasks.clone(), self.texts.clone()],
        }
    }

    /// A collection of all global DPac files in the controller.
    pub fn globals(&self) -> FileSet {
        FileSet {
            file_sets: vec![self.globals.clone()],
        }
    }

    /// Searches the controller for information about a variable.
    /// Globals are searched as well.
    /// Variable names are case insensitive.
    pub fn lookup_variable(&self, variable_name: &str) -> Option<Variable> {
        let (filename, variable_name_part) = match variable_name.split_once('.') {
            Some((a, b)) => (a, Some(b)),
            None => (variable_name, None),
        };

        if let Some(variable_name_part) = variable_name_part {
            match self.files().get(filename) {
                None => {}
                Some(file) => match file.get(variable_name_part) {
                    None => {}
                    Some(variable) => return Some(variable),
                },
            }
        }

        for file in self.globals().iter() {
            match file.get(variable_name) {
                None => continue,
                Some(variable) => return Some(variable),
            }
        }

        None
    }
}

impl PartialEq for Controller {
    fn eq(&self, other: &Self) -> bool {
        if self.address != other.address || self.require_password != other.require_password || self.system_password != other.system_password {
            return false;
        }

        if self.tasks.len() != other.tasks.len()
            || self.dpacs.len() != other.dpacs.len()
            || self.texts.len() != other.texts.len()
            || self.globals.len() != other.globals.len()
        {
            return false;
        }

        for name in self.globals.keys() {
            if !other.globals.contains_key(name) {
                return false;
            }
        }

        for (fileset, fileset_o) in [(&self.tasks, &other.tasks), (&self.dpacs, &other.dpacs), (&self.texts, &other.texts)] {
            for (name, (ln, file)) in fileset.iter() {
                let Some((ln_o, file_o)) = fileset_o.get(name) else {
                    return false;
                };
                if ln != ln_o || file.hash != file_o.hash {
                    return false;
                }
            }
        }

        true
    }
}
