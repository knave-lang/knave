use std::rc::Rc;

use super::imported_names::ImportedNames;
use super::name_entries::NameEntries;
use super::saved_exports::SavedExports;
use super::saved_names::SavedNames;
use super::{Scope, ScopeSet};
use crate::core::{BindingId, Namespace};

#[derive(Default)]
pub struct PreludeNames {
    saved: SavedNames,
    imported: ImportedNames,
}

impl PreludeNames {
    pub fn adopt_saved(&mut self, saved: SavedNames) {
        self.saved = saved;
    }

    pub fn import(&mut self, scope: Scope, exports: SavedExports) {
        self.imported.add(scope, exports);
    }

    pub(super) fn imported_with_scopes(
        &self,
        scopes: &ScopeSet,
        name: &str,
        namespace: Namespace,
    ) -> Option<BindingId> {
        self.imported.bound_with_scopes(scopes, name, namespace)
    }

    pub(super) fn entries_seen_from<'t, 's>(
        &'t self,
        scopes: &'s ScopeSet,
        name: Rc<str>,
        namespace: Namespace,
    ) -> impl Iterator<Item = &'t NameEntries> + use<'t, 's> {
        let saved = self.saved.entries_seen_from(scopes, &name, namespace);
        self.imported
            .entries_seen_from(scopes, name, namespace)
            .chain(saved)
    }

    pub(super) fn qualified_ending_in(
        &self,
        last_segment: Rc<str>,
        namespace: Namespace,
    ) -> &[Rc<str>] {
        self.imported.qualified_ending_in(last_segment, namespace)
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        self.imported.all().chain(self.saved.all())
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        self.imported
            .all_of(namespace)
            .chain(self.saved.all_of(namespace))
    }

    #[cfg(test)]
    pub(super) fn is_saved_decoded(&self, name: &str, namespace: Namespace) -> bool {
        self.saved.is_decoded(name, namespace)
    }

    #[cfg(test)]
    pub(crate) fn imported_decoded_count(&self) -> usize {
        self.imported.decoded_count()
    }

    #[cfg(test)]
    pub(crate) fn imported_module_count(&self) -> usize {
        self.imported.module_count()
    }
}
