use std::path::{Component, Path, PathBuf};

use super::crate_name::CrateName;
use crate::language::Language;

pub struct BuildDirectory {
    root: PathBuf,
    working_directory: PathBuf,
}

impl BuildDirectory {
    pub const PARENT: &str = "target";
    const OUTSIDE_WORKING_DIRECTORY: &str = "outside";

    pub fn new(root: PathBuf, working_directory: PathBuf) -> Self {
        Self {
            root,
            working_directory,
        }
    }

    pub fn root() -> PathBuf {
        Path::new(Self::PARENT).join(Language::NAME)
    }

    pub fn for_entry(&self, entry_directory: &Path, crate_name: &CrateName) -> PathBuf {
        crate_name.path_in(&self.root.join(self.mirror(entry_directory)))
    }

    fn mirror(&self, entry_directory: &Path) -> PathBuf {
        match entry_directory.strip_prefix(&self.working_directory) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => {
                Path::new(Self::OUTSIDE_WORKING_DIRECTORY).join(Self::without_root(entry_directory))
            }
        }
    }

    fn without_root(directory: &Path) -> PathBuf {
        directory
            .components()
            .filter(|component| matches!(component, Component::Normal(_)))
            .collect()
    }
}
