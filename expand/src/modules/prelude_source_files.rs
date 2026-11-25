use super::embedded_files::EmbeddedModules;
use super::registry::ModuleRecord;
use crate::core::ModulePath;
use crate::language::Language;
use crate::scope::{ModulePathBytes, SavedModulePaths};
use crate::source::EmbeddedSources;

#[derive(serde::Serialize)]
pub struct PreludeSourceBytes {
    paths: ModulePathBytes,
}

#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct PreludeSourceFiles {
    paths: SavedModulePaths,
}

impl PreludeSourceFiles {
    const FIRST: usize = 0;

    pub fn of(records: &[ModuleRecord]) -> PreludeSourceBytes {
        let embedded: Vec<&str> = records
            .iter()
            .map(|record| record.path().as_str())
            .filter(|path| EmbeddedModules::contains(&ModulePath::new(*path)))
            .collect();
        PreludeSourceBytes {
            paths: ModulePathBytes::of(embedded.into_iter()),
        }
    }

    pub fn first_name(&self) -> String {
        self.name(Self::FIRST)
    }
}

impl EmbeddedSources for PreludeSourceFiles {
    fn count(&self) -> usize {
        self.paths.count()
    }

    fn name(&self, position: usize) -> String {
        Language::file_name(self.paths.path(position))
    }

    fn text(&self, position: usize) -> &'static str {
        let path = ModulePath::new(self.paths.path(position));
        EmbeddedModules::text(&path).expect("a saved prelude source is an embedded module")
    }
}
