use std::fs;
use std::path::PathBuf;

use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;

pub struct NewFile {
    pub path: PathBuf,
    pub text: String,
}

impl NewFile {
    pub fn write(&self) -> Result<(), ExitStatus> {
        fs::create_dir_all(ProgramFile::directory_of(&self.path))
            .and_then(|()| fs::write(&self.path, &self.text))
            .map_err(|error| ProgramFile::report_unwritable(&self.path, &error))
    }
}
