use super::embedded_files::EmbeddedModules;
use super::registry::ModuleRecord;
use crate::core::ModulePath;
use crate::language::Language;
use crate::scope::{PositionBytes, SavedModulePaths, SavedPositions};
use crate::source::EmbeddedSources;

#[derive(serde::Serialize)]
pub struct PreludeSourceBytes {
    positions: PositionBytes,
}

#[derive(Clone, Copy)]
pub struct PreludeSourceFiles {
    paths: SavedModulePaths,
    positions: SavedPositions,
}

impl PreludeSourceFiles {
    const FIRST: usize = 0;

    pub fn new(paths: SavedModulePaths, positions: SavedPositions) -> Self {
        Self { paths, positions }
    }

    pub fn of(records: &[ModuleRecord]) -> PreludeSourceBytes {
        let embedded = records
            .iter()
            .enumerate()
            .filter(|(_, record)| EmbeddedModules::contains(record.path()))
            .map(|(position, _)| position);
        PreludeSourceBytes {
            positions: PositionBytes::of(embedded),
        }
    }

    pub fn first_name(&self) -> String {
        self.name(Self::FIRST)
    }

    fn path(&self, index: usize) -> &'static str {
        self.paths.path(self.positions.at(index))
    }
}

impl EmbeddedSources for PreludeSourceFiles {
    fn count(&self) -> usize {
        self.positions.count()
    }

    fn name(&self, index: usize) -> String {
        Language::file_name(self.path(index))
    }

    fn text(&self, index: usize) -> &'static str {
        let path = ModulePath::new(self.path(index));
        EmbeddedModules::text(&path).expect("a saved prelude source is an embedded module")
    }
}
