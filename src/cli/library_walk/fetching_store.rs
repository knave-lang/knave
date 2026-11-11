use std::path::PathBuf;

use super::library_store::LibraryStore;
use super::user_git_store::UserGitStore;
use crate::build::StoreError;

#[derive(Default)]
pub struct FetchingStore {
    git: UserGitStore,
}

impl LibraryStore for FetchingStore {
    fn checkout(&mut self, url: &str, commit: &str) -> Result<PathBuf, StoreError> {
        self.git.opened()?.checkout(url, commit)
    }
}
