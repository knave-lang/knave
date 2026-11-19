use std::path::{Path, PathBuf};

use crate::cli::program_file::ProgramFile;
use crate::language::Language;

pub struct ProjectRoot;

impl ProjectRoot {
    pub fn canonical_of(directory: &Path) -> Option<PathBuf> {
        let start = ProgramFile::canonical(directory)?;
        let file_name = Language::project_file_name();
        start
            .ancestors()
            .find(|ancestor| ancestor.join(&file_name).is_file())
            .map(Path::to_path_buf)
    }
}
