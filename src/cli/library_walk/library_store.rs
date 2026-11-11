use std::path::PathBuf;

use crate::build::StoreError;

pub trait LibraryStore {
    fn checkout(&mut self, url: &str, commit: &str) -> Result<PathBuf, StoreError>;
}
