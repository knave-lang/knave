use std::fs;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;

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

    pub fn write(&self) -> Result<(), ExitStatus> {
        fs::create_dir_all(ProgramFile::directory_of(&self.path))
            .and_then(|()| fs::write(&self.path, &self.text))
            .map_err(|error| ProgramFile::report_unwritable(&self.path, &error))
    }
}
