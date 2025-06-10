use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use unicase::UniCase;

use super::super::{controller_loader::LoadMode, variable::VariableKind};
use super::variable_internal::{StandardVariable, VerboseVariable};

pub type VerboseVariableMap = HashMap<Arc<UniCase<String>>, VerboseVariable>;
pub type StandardVariableMap = HashMap<u64, StandardVariable>;

pub enum VariableMap {
    Verbose(LoadMode, VerboseVariableMap),
    Standard(#[allow(unused)] LoadMode, StandardVariableMap),
}

impl VariableMap {
    pub fn new(mode: LoadMode) -> Self {
        match mode {
            LoadMode::WithNames | LoadMode::WithNamesAndComments => Self::Verbose(mode, HashMap::new()),
            LoadMode::HashedNames => Self::Standard(mode, HashMap::new()),
        }
    }

    pub fn hash_key(key: &UniCase<String>) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

impl VariableMap {
    pub fn is_empty(&self) -> bool {
        match self {
            VariableMap::Verbose(_, hash_map) => hash_map.is_empty(),
            VariableMap::Standard(_, hash_map) => hash_map.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            VariableMap::Verbose(_, hash_map) => hash_map.len(),
            VariableMap::Standard(_, hash_map) => hash_map.len(),
        }
    }

    pub fn insert(&mut self, name: UniCase<String>, kind: VariableKind, offset: u32, comment: Option<&str>) {
        match self {
            VariableMap::Verbose(mode, hash_map) => {
                hash_map.insert(
                    name.into(),
                    VerboseVariable {
                        kind,
                        offset,
                        comment: match mode {
                            LoadMode::WithNamesAndComments => comment.map(|s| s.to_string().into()),
                            _ => None,
                        },
                    },
                );
            }
            VariableMap::Standard(_, hash_map) => {
                let key = VariableMap::hash_key(&name);
                hash_map.insert(key, StandardVariable::new(kind, offset));
            }
        }
    }
}
