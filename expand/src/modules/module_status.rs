use std::cell::Cell;

use crate::scope::{IntroductionScopes, ScopeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum LoadStatus {
    Loading,
    Loaded,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModuleStatus {
    load: LoadStatus,
    #[serde(skip)]
    path_readers: Cell<Option<Box<PathReaders>>>,
}

#[derive(Default)]
struct PathReaders(Vec<ScopeSet>);

impl ModuleStatus {
    pub(super) fn loading() -> Self {
        Self {
            load: LoadStatus::Loading,
            path_readers: Cell::default(),
        }
    }

    pub(super) fn finish_loading(&mut self) {
        self.load = LoadStatus::Loaded;
    }

    pub fn is_loading(&self) -> bool {
        self.load == LoadStatus::Loading
    }

    pub(crate) fn note_path_read(&self, written: &ScopeSet, introductions: &IntroductionScopes) {
        let mut readers = self.path_readers.take().unwrap_or_default();
        let stored = |read: &ScopeSet| introductions.equal_without_introductions(written, read);
        if !readers.0.iter().any(stored) {
            let mut read = written.clone();
            introductions.remove_from(&mut read);
            readers.0.push(read);
        }
        self.path_readers.set(Some(readers));
    }

    pub fn is_read_by_path(&self, reader: &ScopeSet) -> bool {
        let readers = self.path_readers.take();
        let read = readers
            .as_ref()
            .is_some_and(|readers| readers.0.iter().any(|written| reader.is_subset_of(written)));
        self.path_readers.set(readers);
        read
    }
}

#[cfg(test)]
mod tests {
    use super::{LoadStatus, ModuleStatus, PathReaders};
    use crate::scope::{ScopeKind, ScopeSet, Scopes};

    #[test]
    fn a_module_reads_by_path_when_a_path_it_wrote_carries_its_scopes() {
        let mut scopes = Scopes::default();
        let [base, body, other, extra] = [(); 4].map(|()| scopes.fresh(ScopeKind::Module));
        let status = ModuleStatus::loading();
        status.note_path_read(&ScopeSet::of(&[base, body, extra]), &scopes.introductions());
        assert!(status.is_read_by_path(&ScopeSet::of(&[base, body])));
        assert!(!status.is_read_by_path(&ScopeSet::of(&[base, other])));
    }

    #[test]
    fn path_reads_of_one_module_under_two_expansions_store_one_set() {
        let mut scopes = Scopes::default();
        let [base, body] = [(); 2].map(|()| scopes.fresh(ScopeKind::Module));
        let [first, second] = [(); 2].map(|()| scopes.fresh(ScopeKind::Introduction));
        let status = ModuleStatus::loading();
        for expansion in [first, second] {
            status.note_path_read(
                &ScopeSet::of(&[base, body, expansion]),
                &scopes.introductions(),
            );
        }
        let readers = status
            .path_readers
            .take()
            .expect("a path read stores a set");
        assert_eq!(readers.0, [ScopeSet::of(&[base, body])]);
    }

    #[test]
    fn a_finished_module_is_no_longer_loading() {
        let mut status = ModuleStatus::loading();
        status.finish_loading();
        assert!(!status.is_loading());
    }

    #[test]
    fn a_module_that_no_path_names_adds_one_pointer_to_its_status() {
        assert_eq!(
            std::mem::size_of::<ModuleStatus>(),
            std::mem::size_of::<(LoadStatus, Box<PathReaders>)>()
        );
    }
}
