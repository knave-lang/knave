use std::path::{Path, PathBuf};

use crate::cli::exit_status::ExitStatus;
use crate::cli::program_file::ProgramFile;
use crate::cli::working_directory::WorkingDirectory;
use crate::language::Language;

pub struct ProjectRoot;

impl ProjectRoot {
    pub fn shown_of(directory: &Path) -> Result<Option<PathBuf>, ExitStatus> {
        match Self::canonical_of(directory) {
            Some(root) => Ok(Some(Self::shown(&root)?)),
            None => Ok(None),
        }
    }

    pub fn canonical_of(directory: &Path) -> Option<PathBuf> {
        let start = ProgramFile::canonical(directory)?;
        let file_name = Language::project_file_name();
        start
            .ancestors()
            .find(|ancestor| ancestor.join(&file_name).is_file())
            .map(Path::to_path_buf)
    }

    fn shown(root: &Path) -> Result<PathBuf, ExitStatus> {
        let working_directory = WorkingDirectory::resolved()?;
        Ok(root
            .strip_prefix(&working_directory)
            .unwrap_or(root)
            .to_path_buf())
    }
}
