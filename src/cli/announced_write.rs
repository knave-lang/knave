use std::path::Path;

use super::exit_status::ExitStatus;
use super::new_file::NewFile;
use super::standard_error::StandardError;

#[derive(Debug, PartialEq)]
pub struct AnnouncedWrite {
    file: NewFile,
    announcement: String,
}

impl AnnouncedWrite {
    pub fn new(file: NewFile, announcement: String) -> Self {
        Self { file, announcement }
    }

    pub fn creation(file: NewFile, shown: &Path) -> Self {
        let announcement = format!("wrote `{}`", shown.display());
        Self::new(file, announcement)
    }

    pub fn apply(&self) -> Result<(), ExitStatus> {
        self.file.write()?;
        StandardError::write_line(format_args!("{}", self.announcement));
        Ok(())
    }
}
