use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct DirectLibrary {
    pub name: String,
    pub root: PathBuf,
    pub exported_tasks: Vec<String>,
    pub fetched: bool,
}
