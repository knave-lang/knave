use std::path::PathBuf;

use crate::core::ModulePath;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeclarationKey {
    InFile(PathBuf, String),
    InModule(ModulePath, String),
}
