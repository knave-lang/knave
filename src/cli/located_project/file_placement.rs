mod tree_scoping;

use std::path::{Path, PathBuf};

use crate::cli::program_file::ProgramFile;
use crate::modules::{LibraryTable, SearchScope};
use tree_scoping::ProjectFolders;
pub use tree_scoping::TreeScoping;

pub struct FilePlacement<'p> {
    source_root: PathBuf,
    tests_directory: PathBuf,
    tasks_directory: PathBuf,
    libraries: &'p LibraryTable,
}

impl<'p> FilePlacement<'p> {
    pub fn new(root: &Path, source_root: PathBuf, libraries: &'p LibraryTable) -> Self {
        Self {
            source_root,
            tests_directory: root.join(SearchScope::TESTS_DIRECTORY),
            tasks_directory: root.join(SearchScope::TASKS_DIRECTORY),
            libraries,
        }
    }

    pub fn tests_directory(&self) -> &Path {
        &self.tests_directory
    }

    pub fn tasks_directory(&self) -> &Path {
        &self.tasks_directory
    }

    pub fn trees(&self) -> TreeScoping<'_> {
        let folders = ProjectFolders {
            source_root: &self.source_root,
            tests_directory: &self.tests_directory,
            tasks_directory: &self.tasks_directory,
        };
        TreeScoping::new(folders, self.libraries)
    }

    pub fn placed_file(&self, path: PathBuf) -> ProgramFile {
        let is_test = ProgramFile::relative_module(&path, &self.tests_directory).is_some();
        let file = self.file_within(path);
        if is_test {
            return self.trees().test_file(file);
        }
        file
    }

    pub fn file_within(&self, path: PathBuf) -> ProgramFile {
        if ProgramFile::relative_module(&path, &self.tasks_directory).is_some() {
            return self
                .trees()
                .task_file(ProgramFile::within(path, self.tasks_directory.clone()));
        }
        let scope = SearchScope::in_project(&self.tasks_directory);
        ProgramFile::inside_or_beside(path, self.source_root.clone())
            .with_libraries(self.libraries.clone())
            .within_scope(scope)
    }
}
