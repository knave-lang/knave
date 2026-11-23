use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum EntryModule {
    Written(PathBuf),
    Default(PathBuf),
}

impl EntryModule {
    pub fn path(&self) -> &Path {
        match self {
            Self::Written(path) | Self::Default(path) => path,
        }
    }

    pub fn is_written(&self) -> bool {
        matches!(self, Self::Written(_))
    }
}
