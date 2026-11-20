use std::path::PathBuf;

use crate::core::ModulePath;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeclarationKey {
    InFile(PathBuf, String),
    InModule(ModulePath, String),
}

impl DeclarationKey {
    pub fn new(file: Option<PathBuf>, module: &ModulePath, name: String) -> Self {
        match file {
            Some(file) => Self::InFile(file, name),
            None => Self::InModule(module.clone(), name),
        }
    }
}
