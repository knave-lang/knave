use crate::core::{ModuleId, ModulePath};
use crate::scope::SavedModulePaths;

#[derive(Clone, Copy)]
pub struct PreludeIndex {
    first: ModuleId,
    paths: SavedModulePaths,
}

impl PreludeIndex {
    pub fn new(first: ModuleId, paths: SavedModulePaths) -> Self {
        Self { first, paths }
    }

    pub fn find(&self, path: &ModulePath) -> Option<ModuleId> {
        let position = self.paths.position(path.as_str())?;
        Some(ModuleId::new(self.first.index() + position))
    }
}
