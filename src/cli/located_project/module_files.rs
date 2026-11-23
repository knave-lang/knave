use std::path::Path;

use super::super::program_file::ProgramFile;
use crate::lint::ProjectLibrary;
use crate::project::ProjectModules;

pub struct ModuleFiles<'p> {
    root: &'p Path,
    modules: &'p ProjectModules,
}

impl<'p> ModuleFiles<'p> {
    pub fn new(root: &'p Path, modules: &'p ProjectModules) -> Self {
        Self { root, modules }
    }

    pub fn is_entry(&self, file: &ProgramFile) -> bool {
        let entry = self.root.join(self.modules.entry.path());
        ProgramFile::canonical(&entry) == ProgramFile::canonical(file.path())
    }

    pub fn library(&self) -> ProjectLibrary {
        if self.root.join(&self.modules.library_root).is_file() {
            ProjectLibrary::Published
        } else {
            ProjectLibrary::Unpublished
        }
    }
}
