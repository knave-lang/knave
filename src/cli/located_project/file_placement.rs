mod project_folders;
mod tree_scoping;

use std::path::{Path, PathBuf};

use crate::cli::program_file::ProgramFile;
use crate::cli::taken_module_paths::TakenModulePaths;
use crate::modules::{LibraryTable, SearchScope};
use project_folders::ProjectFolders;
pub use tree_scoping::TreeScoping;

pub struct FilePlacement<'p> {
    folders: ProjectFolders,
    libraries: &'p LibraryTable,
}

impl<'p> FilePlacement<'p> {
    pub fn new(root: &Path, source_root: PathBuf, libraries: &'p LibraryTable) -> Self {
        Self {
            folders: ProjectFolders::new(root, source_root),
            libraries,
        }
    }

    pub fn tests_directory(&self) -> &Path {
        self.folders.tests_directory()
    }

    pub fn tasks_directory(&self) -> &Path {
        self.folders.tasks_directory()
    }

    pub fn taken_module_paths(&self) -> TakenModulePaths<'p> {
        TakenModulePaths::by_the_compiler_and(self.libraries)
    }

    pub fn trees(&self) -> TreeScoping<'_> {
        TreeScoping::new(&self.folders, self.libraries)
    }

    pub fn placed_file(&self, path: PathBuf) -> ProgramFile {
        let is_test = ProgramFile::relative_module(&path, self.tests_directory()).is_some();
        let file = self.file_within(path);
        if is_test {
            return self.trees().test_file(file);
        }
        file
    }

    pub fn file_within(&self, path: PathBuf) -> ProgramFile {
        let tasks_directory = self.tasks_directory();
        if self.trees().holds_task(&path) {
            let file = ProgramFile::within(path, tasks_directory.to_path_buf());
            return self.trees().task_file(file);
        }
        let scope = SearchScope::in_project(tasks_directory);
        ProgramFile::inside_or_beside(path, self.folders.source_root().to_path_buf())
            .with_libraries(self.libraries.clone())
            .within_scope(scope)
    }
}
