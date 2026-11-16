use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::export_table::ExportTable;
use super::module_scopes::ModuleScopes;
use super::module_status::ModuleStatus;
use crate::core::{BindingId, ModuleId, ModulePath, Namespace};
use crate::reader::Symbol;
use crate::scope::ScopeSet;

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

    pub fn adopt(&mut self, record: ModuleRecord) -> ModuleId {
        let id = ModuleId::new(self.records.len());
        self.by_path.insert(record.path.clone(), id);
        self.records.push(record);
        id
    }

    pub fn into_records_after(mut self, id: ModuleId) -> Vec<ModuleRecord> {
        self.records.split_off(id.index() + 1)
    }

    pub fn find(&self, path: &ModulePath) -> Option<ModuleId> {
        self.by_path.get(path).copied()
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
}
