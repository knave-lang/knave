use std::path::{Path, PathBuf};

use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::terminal::Terminal;

pub struct SkippedDirectory {
    build_parent: Option<PathBuf>,
}

impl SkippedDirectory {
    const HIDDEN_PREFIX: char = '.';

    pub fn for_walk_of(walk_root: &Path) -> Self {
        let build_base =
            LocatedProject::canonical_root(walk_root).or_else(|| ProgramFile::canonical(walk_root));
        Self {
            build_parent: build_base.map(|base| base.join(Terminal::BUILD_PARENT)),
        }
    }

    pub fn is_skipped(&self, directory: &Path) -> bool {
        Self::is_hidden(directory) || self.is_build_parent(directory)
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
