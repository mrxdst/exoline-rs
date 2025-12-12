use std::sync::Arc;

use unicase::UniCase;

use super::{
    internal::{file_internal::FileInternal, variable_internal::VariableImpl, variable_map::VariableMap},
    Variable,
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FileKind {
    /// A table of primitive values.
    BPac,
    /// Contains local variables used by the program code.
    Task,
    /// A global list of strings.
    Text,
    /// User or system defined variables.
    VPac,
}

/// A file with a collection of variables.
#[derive(Clone)]
pub struct File {
    pub(crate) file: Arc<FileInternal>,
    pub(crate) file_key: Arc<UniCase<String>>,
    pub(crate) load_number: u8,
}

impl File {
    /// Retrieves a variable in the file.
    pub fn get(&self, variable_name: &str) -> Option<Variable> {
        let variable_key = UniCase::new(variable_name.into());
        match &self.file.variables {
            VariableMap::Verbose(_, variables) => {
                let (variable_key, variable) = variables.get_key_value(&variable_key)?;
                Some(Variable {
                    file: self.file.clone(),
                    file_name: self.file_key.clone(),
                    kind: variable.kind(),
                    offset: variable.offset(),
                    comment: variable.comment(),
                    name: Some(variable_key.clone()),
                    load_number: self.load_number,
                })
            }
            VariableMap::Standard(_, variables) => {
                let hash = VariableMap::hash_key(&variable_key);
                let variable = variables.get(&hash)?;
                Some(Variable {
                    file: self.file.clone(),
                    file_name: self.file_key.clone(),
                    kind: variable.kind(),
                    offset: variable.offset(),
                    comment: variable.comment(),
                    name: None,
                    load_number: self.load_number,
                })
            }
        }
    }

    /// Returns the number of variables in the file.
    pub fn len(&self) -> usize {
        self.file.variables.len()
    }

    /// Returns `true` if the file contains no variables.
    pub fn is_empty(&self) -> bool {
        self.file.variables.is_empty()
    }

    /// Returns the name of the file.
    pub fn name(&self) -> &Arc<UniCase<String>> {
        &self.file_key
    }

    pub fn kind(&self) -> FileKind {
        self.file.kind
    }

    /// Returns the load number for the file.
    pub fn load_number(&self) -> u8 {
        self.load_number
    }

    /// An iterator over all the variables in the file.
    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = Variable> + Send + 'a> {
        match &self.file.variables {
            VariableMap::Verbose(_, variables) => Box::new(variables.iter().map(|(variable_key, variable)| Variable {
                file: self.file.clone(),
                file_name: self.file_key.clone(),
                kind: variable.kind(),
                offset: variable.offset(),
                comment: variable.comment(),
                name: Some(variable_key.clone()),
                load_number: self.load_number(),
            })),
            VariableMap::Standard(_, variables) => Box::new(variables.values().map(|variable| Variable {
                file: self.file.clone(),
                file_name: self.file_key.clone(),
                kind: variable.kind(),
                offset: variable.offset(),
                comment: variable.comment(),
                name: None,
                load_number: self.load_number(),
            })),
        }
    }
}
