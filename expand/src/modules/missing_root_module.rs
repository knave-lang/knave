use std::path::PathBuf;

pub struct MissingRootModule<'t> {
    pub library: &'t str,
    pub root_file: PathBuf,
    pub project_file: PathBuf,
}
