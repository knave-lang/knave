use std::path::{Path, PathBuf};

use crate::cli::exit_status::ExitStatus;
use crate::cli::program_file::ProgramFile;
use crate::cli::working_directory::WorkingDirectory;
use crate::language::Language;

#[derive(PartialEq, Eq)]
pub struct ProjectRoot {
    path: PathBuf,
}

impl ProjectRoot {
    pub fn around(directory: &Path) -> Option<Self> {
        let start = ProgramFile::canonical(directory)?;
        let file_name = Language::project_file_name();
        let path = start
            .ancestors()
            .find(|ancestor| ancestor.join(&file_name).is_file())?
            .to_path_buf();
        Some(Self { path })
    }

    pub fn shown(&self) -> Result<PathBuf, ExitStatus> {
        let working_directory = WorkingDirectory::resolved()?;
        Ok(self
            .path
            .strip_prefix(&working_directory)
            .unwrap_or(&self.path)
            .to_path_buf())
    }

    pub fn project_file(&self) -> PathBuf {
        self.path.join(Language::project_file_name())
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }
}
