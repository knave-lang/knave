use crate::build::{GitStore, StoreError};

#[derive(Default)]
pub struct UserGitStore {
    store: Option<GitStore>,
}

impl UserGitStore {
    pub fn opened(&mut self) -> Result<&GitStore, StoreError> {
        let store = match self.store.take() {
            Some(store) => store,
            None => GitStore::for_user()?,
        };
        Ok(self.store.insert(store))
    }
}
