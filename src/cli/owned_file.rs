use std::path::{Path, PathBuf};

use super::new_file::NewFile;
use super::versioned_text::VersionedText;

pub struct OwnedFile {
    path: PathBuf,
    text: VersionedText,
}

impl OwnedFile {
    pub fn new(path: PathBuf, text: VersionedText) -> Self {
        Self { path, text }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn current_text(&self) -> String {
        self.text.filled()
    }

    pub fn new_file(&self, root: &Path) -> NewFile {
        NewFile::new(root.join(&self.path), self.current_text())
    }
}
