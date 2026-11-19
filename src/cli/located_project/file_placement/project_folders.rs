use std::path::{Path, PathBuf};

use crate::modules::SearchScope;

pub struct ProjectFolders {
    root: PathBuf,
    source_root: PathBuf,
    tests_directory: PathBuf,
    tasks_directory: PathBuf,
}

impl ProjectFolders {
    pub fn new(root: &Path, source_root: PathBuf) -> Self {
        Self {
            root: root.to_path_buf(),
            source_root,
            tests_directory: root.join(SearchScope::TESTS_DIRECTORY),
            tasks_directory: root.join(SearchScope::TASKS_DIRECTORY),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    pub fn tests_directory(&self) -> &Path {
        &self.tests_directory
    }

    pub fn tasks_directory(&self) -> &Path {
        &self.tasks_directory
    }
}
