use std::path::PathBuf;

use super::library_store::LibraryStore;
use super::user_git_store::UserGitStore;
use crate::build::StoreError;

#[derive(Default)]
pub struct CacheOnlyStore {
    git: UserGitStore,
}

impl LibraryStore for CacheOnlyStore {
    fn checkout(&mut self, _url: &str, commit: &str) -> Result<PathBuf, StoreError> {
        self.git
            .opened()?
            .cached(commit)
            .ok_or(StoreError::NotCached)
    }
}
