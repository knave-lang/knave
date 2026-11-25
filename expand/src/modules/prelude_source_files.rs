use super::embedded_files::EmbeddedModules;
use super::registry::ModuleRecord;
use crate::language::Language;
use crate::scope::{PositionBytes, SavedModulePaths, SavedPositions};
use crate::source::EmbeddedSources;

#[derive(serde::Serialize)]
pub struct PreludeSourceBytes {
    positions: PositionBytes,
}

impl PreludeSourceBytes {
    pub fn of(records: &[ModuleRecord]) -> Self {
        let embedded = records
            .iter()
            .enumerate()
            .filter(|(_, record)| EmbeddedModules::contains(record.path()))
            .map(|(position, _)| position);
        Self {
            positions: PositionBytes::of(embedded),
        }
    }
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
        EmbeddedModules::text_of_path(self.path(index))
            .expect("a saved prelude source is an embedded module")
    }
}
