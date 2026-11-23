use std::path::{Path, PathBuf};

use super::announced_write::AnnouncedWrite;
use super::exit_status::ExitStatus;
use super::line_ending::LineEnding;
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

    pub fn current_text(&self) -> String {
        self.text.filled()
    }

    pub fn new_file(&self, root: &Path) -> NewFile {
        NewFile::new(root.join(&self.path), self.current_text())
    }

    pub fn exists_in(&self, root: &Path) -> bool {
        root.join(&self.path).exists()
    }

    pub fn pending_write(&self, root: &Path) -> Result<Option<AnnouncedWrite>, ExitStatus> {
        let file = self.new_file(root);
        match file.written_bytes()? {
            None => Ok(Some(AnnouncedWrite::creation(file, &self.path))),
            Some(written) if LineEnding::with_lf_endings(&written) == file.text() => Ok(None),
            Some(_) => {
                let announcement = format!("rewrote `{}`", self.path.display());
                Ok(Some(AnnouncedWrite::new(file, announcement)))
            }
        }
    }
}
