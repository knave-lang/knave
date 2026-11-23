use super::exit_status::ExitStatus;
use super::new_file::NewFile;

pub struct AnnouncedWrite {
    file: NewFile,
    announcement: String,
}

impl AnnouncedWrite {
    pub fn new(file: NewFile, announcement: String) -> Self {
        Self { file, announcement }
    }

    pub fn apply(&self) -> Result<(), ExitStatus> {
        self.file.write_announced(&self.announcement)
    }
}
