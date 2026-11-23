use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::diagnostic_output::StandardErrorMessages;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::unreadable_file::UnreadableFile;

pub struct NewFile {
    path: PathBuf,
    text: String,
}

impl NewFile {
    pub fn new(path: PathBuf, text: String) -> Self {
        Self { path, text }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn with_text(self, text: String) -> Self {
        Self { text, ..self }
    }

    pub fn written_bytes(&self) -> Result<Option<Vec<u8>>, ExitStatus> {
        if !self.path.exists() {
            return Ok(None);
        }
        fs::read(&self.path)
            .map(Some)
            .map_err(|error| self.report_unreadable(&error))
    }

    pub fn report_unreadable(&self, error: &io::Error) -> ExitStatus {
        UnreadableFile::new(&self.path, error).report_to(&StandardErrorMessages)
    }

    pub fn write(&self) -> Result<(), ExitStatus> {
        fs::create_dir_all(ProgramFile::directory_of(&self.path))
            .and_then(|()| fs::write(&self.path, &self.text))
            .map_err(|error| ProgramFile::report_unwritable(&self.path, &error))
    }
}
