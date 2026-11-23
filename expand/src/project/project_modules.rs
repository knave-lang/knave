use std::path::PathBuf;

#[derive(Debug)]
pub struct ProjectModules {
    pub entry: PathBuf,
    pub library_root: PathBuf,
    pub entry_written: bool,
}
