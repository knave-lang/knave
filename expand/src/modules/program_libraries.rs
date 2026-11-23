use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct ProgramLibraries {
    pub visible: Vec<String>,
    pub checkout: Option<PathBuf>,
}
