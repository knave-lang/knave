use super::sources::ModuleSources;
use crate::core::ModulePath;

pub struct ModuleOrigin<'m> {
    path: &'m ModulePath,
    library: Option<&'m str>,
}

impl<'m> ModuleOrigin<'m> {
    pub fn new(path: &'m ModulePath, library: Option<&'m str>) -> Self {
        Self { path, library }
    }

    pub fn is_project(&self) -> bool {
        ModuleSources::is_written(self.path) && self.library.is_none()
    }
}
