use super::prelude_index::PreludeIndex;
use super::registry::{ModuleRecord, ModuleRegistry};
use crate::core::ModuleId;
use crate::scope::{ModulePathBytes, SavedModulePaths};

pub struct BuiltPreludeRecords {
    records: Vec<ModuleRecord>,
}

impl BuiltPreludeRecords {
    pub fn of(records: Vec<ModuleRecord>) -> Self {
        Self { records }
    }
}

impl serde::Serialize for BuiltPreludeRecords {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let paths = self.records.iter().map(|record| record.path().as_str());
        (&self.records, ModulePathBytes::of(paths)).serialize(serializer)
    }
}

#[derive(Default, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct PreludeRecords {
    records: Vec<ModuleRecord>,
    paths: SavedModulePaths,
}

impl PreludeRecords {
    pub fn index(&self, first: ModuleId) -> PreludeIndex {
        PreludeIndex::new(first, self.paths)
    }

    #[cfg(test)]
    pub fn records(&self) -> impl Iterator<Item = &ModuleRecord> {
        self.records.iter()
    }

    pub fn install_into(self, registry: &mut ModuleRegistry) {
        registry.adopt_saved(self.records, self.paths);
    }
}
