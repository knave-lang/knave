use std::path::PathBuf;

use crate::build::{GitStore, StoreError};

pub struct LibraryStore {
    store: Option<GitStore>,
    reach: StoreReach,
}

enum StoreReach {
    Fetching,
    CacheOnly,
}

impl LibraryStore {
    pub fn fetching() -> Self {
        Self {
            store: None,
            reach: StoreReach::Fetching,
        }
    }

    pub fn cache_only() -> Self {
        Self {
            store: None,
            reach: StoreReach::CacheOnly,
        }
    }

    pub fn checkout(&mut self, url: &str, commit: &str) -> Result<PathBuf, StoreError> {
        let store = match self.store.take() {
            Some(store) => store,
            None => GitStore::for_user()?,
        };
        let checked_out = match self.reach {
            StoreReach::Fetching => store.checkout(url, commit),
            StoreReach::CacheOnly => store.cached(commit).ok_or(StoreError::NotCached),
        };
        self.store = Some(store);
        checked_out
    }
}
