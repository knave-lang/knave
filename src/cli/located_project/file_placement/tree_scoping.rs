use std::path::Path;

use super::project_folders::ProjectFolders;
use crate::cli::program_file::ProgramFile;
use crate::modules::{LibraryTable, SearchScope};

pub struct TreeScoping<'s> {
    folders: &'s ProjectFolders,
    libraries: &'s LibraryTable,
}

impl<'s> TreeScoping<'s> {
    pub fn new(folders: &'s ProjectFolders, libraries: &'s LibraryTable) -> Self {
        Self { folders, libraries }
    }

    pub fn holds_task(&self, path: &Path) -> bool {
        ProgramFile::relative_module(path, self.folders.tasks_directory()).is_some()
    }

    pub fn test_file(&self, file: ProgramFile) -> ProgramFile {
        let scope = SearchScope::tests(self.folders.root());
        self.beside_source_root(file).within_scope(scope)
    }

    pub fn task_file(&self, file: ProgramFile) -> ProgramFile {
        let scope = SearchScope::tasks(self.folders.root());
        self.beside_source_root(file).within_scope(scope)
    }

    fn beside_source_root(&self, file: ProgramFile) -> ProgramFile {
        file.with_fallback_root(self.folders.source_root().to_path_buf())
            .with_libraries(self.libraries.clone())
    }
}
