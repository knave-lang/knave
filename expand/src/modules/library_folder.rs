use std::path::PathBuf;

#[derive(Clone)]
pub struct LibraryFolder {
    pub project_file: PathBuf,
    pub source_root: PathBuf,
    pub display_root: PathBuf,
}
