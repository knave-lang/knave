use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::export_table::ExportTable;
use super::module_scopes::ModuleScopes;
use super::module_status::ModuleStatus;
use super::prelude_index::PreludeIndex;
use crate::core::{BindingId, ModuleId, ModulePath, Namespace};
use crate::reader::Symbol;
use crate::scope::{SavedModulePaths, ScopeSet};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct ModuleRecord {
    path: ModulePath,
    scopes: ModuleScopes,
    exports: ExportTable,
    status: ModuleStatus,
}

impl ModuleRecord {
    pub fn path(&self) -> &ModulePath {
        &self.path
    }

    pub fn scopes(&self) -> ModuleScopes {
        self.scopes
    }

    pub fn symbol(&self) -> Symbol {
        Symbol::with_scopes(self.path.as_str(), self.scopes.set())
    }

    pub fn export(&mut self, name: Rc<str>, namespace: Namespace, binding: BindingId) {
        self.exports.insert(name, namespace, binding);
    }

    pub fn exported(&self, name: &str, namespace: Namespace) -> Option<BindingId> {
        self.exports.get(name, namespace)
    }

    pub fn exports(&self) -> &ExportTable {
        &self.exports
    }

    pub fn status(&self) -> &ModuleStatus {
        &self.status
    }
}

#[derive(Default)]
pub struct ModuleRegistry {
    records: Vec<ModuleRecord>,
    by_path: FxHashMap<ModulePath, ModuleId>,
    prelude: Option<PreludeIndex>,
}

impl ModuleRegistry {
    pub fn register(&mut self, path: ModulePath, scopes: ModuleScopes) -> ModuleId {
        self.adopt(ModuleRecord {
            path,
            scopes,
            exports: ExportTable::default(),
            status: ModuleStatus::loading(),
        })
    }

    pub fn adopt_saved(&mut self, records: Vec<ModuleRecord>, paths: SavedModulePaths) {
        let first = ModuleId::new(self.records.len());
        let prelude = PreludeIndex::new(first, paths);
        self.by_path.retain(|path, _| prelude.find(path).is_none());
        self.prelude = Some(prelude);
        self.records.extend(records);
    }

    pub fn into_records_after(mut self, id: ModuleId) -> Vec<ModuleRecord> {
        self.records.split_off(id.index() + 1)
    }

    pub fn find(&self, path: &ModulePath) -> Option<ModuleId> {
        self.by_path
            .get(path)
            .copied()
            .or_else(|| self.prelude?.find(path))
    }

    pub fn finish_loading(&mut self, id: ModuleId) {
        self.records[id.index()].status.finish_loading();
    }

    pub fn loading_paths(&self) -> Vec<&ModulePath> {
        self.records
            .iter()
            .filter(|record| record.status.is_loading())
            .map(ModuleRecord::path)
            .collect()
    }

    pub fn with_body_scopes(&self, scopes: &ScopeSet) -> Option<ModuleId> {
        self.records
            .iter()
            .position(|record| record.scopes.set() == *scopes)
            .map(ModuleId::new)
    }

    pub fn records(&self) -> impl Iterator<Item = &ModuleRecord> {
        self.records.iter()
    }

    pub fn get(&self, id: ModuleId) -> &ModuleRecord {
        &self.records[id.index()]
    }

    pub fn get_mut(&mut self, id: ModuleId) -> &mut ModuleRecord {
        &mut self.records[id.index()]
    }

    fn adopt(&mut self, record: ModuleRecord) -> ModuleId {
        let id = ModuleId::new(self.records.len());
        self.by_path.insert(record.path.clone(), id);
        self.records.push(record);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::{ModuleRecord, ModuleRegistry};
    use crate::core::{ModuleId, ModulePath};
    use crate::modules::ModuleScopes;
    use crate::scope::{ModulePathBytes, SavedModulePaths, ScopeKind, Scopes};

    struct SavedPreludeFixture {
        scopes: Scopes,
    }

    impl SavedPreludeFixture {
        const ENTRY: &str = "geometry";
        const PRELUDE_PATHS: [&str; 2] = ["prelude", "prelude/text"];

        fn new() -> Self {
            Self {
                scopes: Scopes::default(),
            }
        }

        fn installed(&mut self) -> ModuleRegistry {
            self.installed_after(Self::ENTRY)
        }

        fn installed_after(&mut self, entry: &str) -> ModuleRegistry {
            let records = self.prelude_records();
            let mut registry = ModuleRegistry::default();
            self.register(&mut registry, entry);
            registry.adopt_saved(records, Self::saved_paths());
            registry
        }

        fn prelude_records(&mut self) -> Vec<ModuleRecord> {
            let mut built = ModuleRegistry::default();
            let entry = self.register(&mut built, Self::ENTRY);
            for path in Self::PRELUDE_PATHS {
                self.register(&mut built, path);
            }
            built.into_records_after(entry)
        }

        fn saved_paths() -> SavedModulePaths {
            let bytes = bincode::serialize(&ModulePathBytes::of(Self::PRELUDE_PATHS.into_iter()))
                .expect("the prelude's paths save");
            bincode::deserialize(bytes.leak()).expect("the prelude's paths load")
        }

        fn register(&mut self, registry: &mut ModuleRegistry, path: &str) -> ModuleId {
            let scopes = ModuleScopes::new(
                self.scopes.fresh(ScopeKind::Module),
                self.scopes.fresh(ScopeKind::Module),
            );
            registry.register(ModulePath::new(path), scopes)
        }
    }

    #[test]
    fn an_adopted_prelude_maps_no_path_of_its_own() {
        let registry = SavedPreludeFixture::new().installed();
        let mapped: Vec<&str> = registry.by_path.keys().map(ModulePath::as_str).collect();
        assert_eq!(mapped, [SavedPreludeFixture::ENTRY]);
    }

    #[test]
    fn an_adopted_prelude_module_is_found_after_the_entry() {
        let registry = SavedPreludeFixture::new().installed();
        for (offset, path) in SavedPreludeFixture::PRELUDE_PATHS.iter().enumerate() {
            let found = registry.find(&ModulePath::new(*path));
            assert_eq!(found, Some(ModuleId::new(1 + offset)), "{path}");
        }
    }

    #[test]
    fn a_saved_prelude_module_wins_over_an_entry_of_its_path() {
        let registry = SavedPreludeFixture::new().installed_after("prelude/text");
        let found = registry.find(&ModulePath::new("prelude/text"));
        assert_eq!(found, Some(ModuleId::new(2)));
    }
}
