use std::cell::OnceCell;
use std::rc::Rc;
use std::slice;

use rustc_hash::FxHashMap;

use super::name_entries::NameEntries;
use super::name_key::NameKey;
use super::phase::Phase;
use super::saved_exports::SavedExports;
use super::scope_set::SortedIter;
use super::{Scope, ScopeSet};
use crate::core::{BindingId, ModuleId, Namespace};

pub(super) struct ImportedTable {
    base: ModuleId,
    exports: SavedExports,
    modules: Vec<ImportedModule>,
    module_by_scope: FxHashMap<Scope, usize>,
}

struct ImportedModule {
    scope: Scope,
    entries: Vec<OnceCell<Box<NameEntries>>>,
}

impl ImportedTable {
    pub(super) fn new(base: ModuleId, exports: SavedExports) -> Self {
        Self {
            base,
            exports,
            modules: Vec::new(),
            module_by_scope: FxHashMap::default(),
        }
    }

    pub(super) fn is_of(&self, base: ModuleId) -> bool {
        self.base == base
    }

    pub(super) fn add(&mut self, scope: Scope) {
        self.module_by_scope.insert(scope, self.modules.len());
        let entries = (0..self.exports.count()).map(|_| OnceCell::new()).collect();
        self.modules.push(ImportedModule { scope, entries });
    }

    pub(super) fn visit_entries_seen_from<'t>(
        &'t self,
        scopes: &ScopeSet,
        key: NameKey<'_>,
        visit: &mut impl FnMut(&'t NameEntries),
    ) {
        let Some(position) = self.exports.position(key) else {
            return;
        };
        for module in self.modules_seen_from(scopes) {
            visit(self.decoded(module, position));
        }
    }

    pub(super) fn bound_with_scope(&self, scope: Scope, key: NameKey<'_>) -> Option<BindingId> {
        self.module_by_scope.get(&scope)?;
        let (_, _, binding) = self.exports.export(self.exports.position(key)?);
        Some(binding)
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        self.modules.iter().flat_map(move |module| {
            (0..self.exports.count()).map(move |position| {
                let (name, namespace, _) = self.exports.export(position);
                (Rc::from(name), namespace, self.decoded(module, position))
            })
        })
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        self.modules.iter().flat_map(move |module| {
            (0..self.exports.count())
                .filter(move |position| self.exports.export(*position).1 == namespace)
                .map(move |position| self.decoded(module, position))
        })
    }

    #[cfg(test)]
    pub(super) fn module_count(&self) -> usize {
        self.modules.len()
    }

    #[cfg(test)]
    pub(super) fn decoded_count(&self) -> usize {
        let entries = self.modules.iter().flat_map(|module| &module.entries);
        entries.filter(|entry| entry.get().is_some()).count()
    }

    fn modules_seen_from<'t, 's>(&'t self, scopes: &'s ScopeSet) -> SeenModules<'t, 's> {
        if self.modules.len() <= scopes.len() {
            SeenModules::ByModule {
                modules: self.modules.iter(),
                scopes,
            }
        } else {
            SeenModules::ByScope {
                scopes: scopes.iter(),
                table: self,
            }
        }
    }

    fn decoded<'t>(&'t self, module: &'t ImportedModule, position: usize) -> &'t NameEntries {
        module.entries[position].get_or_init(|| Box::new(self.decode(module.scope, position)))
    }

    fn decode(&self, scope: Scope, position: usize) -> NameEntries {
        let (_, _, binding) = self.exports.export(position);
        let mut scopes = ScopeSet::default();
        scopes.add(scope);
        let mut entries = NameEntries::default();
        entries
            .bind(&scopes, binding, Phase::Runtime)
            .expect("an empty entry list takes any binding");
        entries
    }
}

enum SeenModules<'t, 's> {
    ByModule {
        modules: slice::Iter<'t, ImportedModule>,
        scopes: &'s ScopeSet,
    },
    ByScope {
        scopes: SortedIter<'s>,
        table: &'t ImportedTable,
    },
}

impl<'t> Iterator for SeenModules<'t, '_> {
    type Item = &'t ImportedModule;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::ByModule { modules, scopes } => {
                modules.find(|module| scopes.contains(module.scope))
            }
            Self::ByScope { scopes, table } => {
                let module = scopes.find_map(|scope| table.module_by_scope.get(&scope))?;
                Some(&table.modules[*module])
            }
        }
    }
}
