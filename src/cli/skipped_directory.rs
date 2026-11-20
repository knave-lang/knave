use std::path::{Path, PathBuf};

use super::build_directory::BuildDirectory;
use super::located_project::ProjectRoot;
use super::program_file::ProgramFile;
use super::skip_reason::SkipReason;

pub struct SkippedDirectory {
    build_parent: Option<PathBuf>,
}

impl SkippedDirectory {
    const HIDDEN_PREFIX: char = '.';

    pub fn for_walk_of(walk_root: &Path) -> Self {
        let build_base = ProjectRoot::around(walk_root)
            .map(ProjectRoot::into_path)
            .or_else(|| ProgramFile::canonical(walk_root));
        Self {
            build_parent: build_base.map(|base| base.join(BuildDirectory::PARENT)),
        }
    }

    pub fn is_skipped(&self, directory: &Path) -> bool {
        self.skip_reason(directory).is_some()
    }

    pub fn skip_reason(&self, directory: &Path) -> Option<SkipReason> {
        if Self::is_hidden(directory) {
            return Some(SkipReason::HiddenFolder);
        }
        self.is_build_parent(directory)
            .then_some(SkipReason::BuildFolder)
    }

    fn is_hidden(directory: &Path) -> bool {
        directory
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(Self::HIDDEN_PREFIX))
    }

    fn is_build_parent(&self, directory: &Path) -> bool {
        self.build_parent.is_some() && ProgramFile::canonical(directory) == self.build_parent
    }
}
