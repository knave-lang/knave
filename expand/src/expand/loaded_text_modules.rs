use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::module_load_site::ModuleLoadSite;
use super::second_module_path::SecondModulePath;
use super::text_module_load::TextModuleLoad;
use super::text_module_record::TextModuleRecord;
use crate::core::{ModuleId, ModulePath};
use crate::modules::{ModuleRegistry, RealPath};
use crate::source::SourceMap;

#[derive(Default)]
pub(super) struct LoadedTextModules {
    owners: FxHashMap<ModulePath, Rc<str>>,
    first_loads: FxHashMap<RealPath, ModuleLoadSite>,
    kept_paths: FxHashMap<ModulePath, ModulePath>,
}

impl LoadedTextModules {
    pub(super) fn record(&mut self, load: TextModuleLoad, sources: &SourceMap) -> TextModuleRecord {
        let first = load
            .file
            .as_ref()
            .and_then(|file| self.first_loads.get(file));
        let record = match first {
            Some(first) => SecondModulePath::new(first.clone(), load.site.clone()).record(sources),
            None => TextModuleRecord::New,
        };
        match &record {
            TextModuleRecord::BindsTo { kept, .. } => {
                self.kept_paths.insert(load.site.path, kept.clone());
            }
            TextModuleRecord::New => {
                if let Some(file) = load.file {
                    self.first_loads.insert(file, load.site.clone());
                }
                self.record_owner(load.site);
            }
            TextModuleRecord::Separate => self.record_owner(load.site),
        }
        record
    }

    fn record_owner(&mut self, site: ModuleLoadSite) {
        if let Some(owner) = site.owner {
            self.owners.insert(site.path, owner);
        }
    }

    pub(super) fn find_module(
        &self,
        modules: &ModuleRegistry,
        path: &ModulePath,
    ) -> Option<ModuleId> {
        modules
            .find(path)
            .or_else(|| modules.find(self.kept_paths.get(path)?))
    }

    pub(super) fn owner_of(&self, path: &ModulePath) -> Option<Rc<str>> {
        self.owners.get(path).cloned()
    }
}
