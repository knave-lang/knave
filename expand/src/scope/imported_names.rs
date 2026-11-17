use std::cell::OnceCell;
use std::rc::Rc;

use super::ScopeSet;
use super::name_entries::NameEntries;
use super::phase::Phase;
use super::saved_exports::SavedExports;
use crate::core::{BindingId, Namespace};

#[derive(Default)]
pub(super) struct ImportedNames {
    modules: Vec<ImportedModule>,
}

struct ImportedModule {
    scopes: ScopeSet,
    exports: SavedExports,
    entries: Vec<OnceCell<Box<NameEntries>>>,
}

impl ImportedNames {
    pub(super) fn add(&mut self, scopes: ScopeSet, exports: SavedExports) {
        let entries = (0..exports.count()).map(|_| OnceCell::new()).collect();
        self.modules.push(ImportedModule {
            scopes,
            exports,
            entries,
        });
    }

    pub(super) fn entries_seen_from<'t>(
        &'t self,
        scopes: &ScopeSet,
        name: &str,
        namespace: Namespace,
    ) -> impl Iterator<Item = &'t NameEntries> {
        self.modules
            .iter()
            .filter(|module| module.scopes.is_subset_of(scopes))
            .filter_map(move |module| module.entries(name, namespace))
    }

    pub(super) fn bound_with_scopes(
        &self,
        scopes: &ScopeSet,
        name: &str,
        namespace: Namespace,
    ) -> Option<BindingId> {
        self.modules
            .iter()
            .filter(|module| module.scopes == *scopes)
            .find_map(|module| module.exports.get(name, namespace))
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        self.modules.iter().flat_map(ImportedModule::all)
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        self.modules
            .iter()
            .flat_map(move |module| module.all_of(namespace))
    }

    #[cfg(test)]
    pub(super) fn decoded_count(&self) -> usize {
        let entries = self.modules.iter().flat_map(|module| &module.entries);
        entries.filter(|entry| entry.get().is_some()).count()
    }
}

impl ImportedModule {
    fn entries(&self, name: &str, namespace: Namespace) -> Option<&NameEntries> {
        let position = self.exports.position(name, namespace)?;
        Some(self.decoded(position))
    }

    fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        (0..self.entries.len()).map(|position| {
            let (name, namespace) = self.exports.key(position);
            (Rc::from(name), namespace, self.decoded(position))
        })
    }

    fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        (0..self.entries.len())
            .filter(move |position| self.exports.key(*position).1 == namespace)
            .map(|position| self.decoded(position))
    }

    fn decoded(&self, position: usize) -> &NameEntries {
        self.entries[position].get_or_init(|| Box::new(self.decode(position)))
    }

    fn decode(&self, position: usize) -> NameEntries {
        let mut entries = NameEntries::default();
        entries
            .bind(&self.scopes, self.exports.binding(position), Phase::Runtime)
            .expect("an empty entry list takes any binding");
        entries
    }
}
