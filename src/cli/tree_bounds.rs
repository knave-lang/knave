use std::path::{Path, PathBuf};

use super::program_file::ProgramFile;

pub struct TreeBounds {
    root: Option<PathBuf>,
}

impl TreeBounds {
    pub fn of(root: &Path) -> Self {
        Self {
            root: ProgramFile::canonical(root),
        }
    }

    pub fn inside(&self, directory: &Path) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        ProgramFile::canonical(directory).filter(|directory| directory.starts_with(root))
    }
}
