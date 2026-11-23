use std::fs;
use std::path::{Path, PathBuf};

use super::announced_write::AnnouncedWrite;
use super::diagnostic_output::StandardErrorMessages;
use super::exit_status::ExitStatus;
use super::new_file::NewFile;
use super::unreadable_file::UnreadableFile;
use super::versioned_text::VersionedText;

pub struct OwnedFile {
    path: PathBuf,
    text: VersionedText,
}

impl OwnedFile {
    const CRLF: &str = "\r\n";
    const LF: &str = "\n";

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
        if !file.path().exists() {
            let announcement = format!("wrote `{}`", self.path.display());
            return Ok(Some(AnnouncedWrite::new(file, announcement)));
        }
        let written = fs::read(file.path()).map_err(|error| {
            UnreadableFile::new(file.path(), &error).report_to(&StandardErrorMessages)
        })?;
        if Self::with_unix_line_endings(&written) == file.text() {
            return Ok(None);
        }
        let announcement = format!("rewrote `{}`", self.path.display());
        Ok(Some(AnnouncedWrite::new(file, announcement)))
    }

    fn with_unix_line_endings(written: &[u8]) -> String {
        String::from_utf8_lossy(written).replace(Self::CRLF, Self::LF)
    }
}
