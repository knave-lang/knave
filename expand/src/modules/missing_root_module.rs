use std::path::{Path, PathBuf};

pub struct MissingRootModule<'t> {
    pub library: &'t str,
    pub root_file: PathBuf,
    pub project_file: &'t Path,
}
