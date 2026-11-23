use std::path::PathBuf;

use super::entry_module::EntryModule;

#[derive(Debug)]
pub struct ProjectModules {
    pub entry: EntryModule,
    pub library_root: PathBuf,
}
