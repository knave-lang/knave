use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::module_names::ModuleNames;
use super::program_file::ProgramFile;
use super::skip_reason::SkipReason;
use super::skipped_directory::SkippedDirectory;
use super::tree_bounds::TreeBounds;
use crate::language::Language;

pub struct TreeWalk<'w> {
    root: &'w Path,
    skipped: &'w SkippedDirectory,
    bounds: Option<TreeBounds>,
    sources: Vec<PathBuf>,
    hidden_folders: Vec<PathBuf>,
    project_files: Vec<PathBuf>,
}

impl<'w> TreeWalk<'w> {
    pub fn new(root: &'w Path, skipped: &'w SkippedDirectory, bounds: Option<TreeBounds>) -> Self {
        Self {
            root,
            skipped,
            bounds,
            sources: Vec::new(),
            hidden_folders: Vec::new(),
            project_files: Vec::new(),
        }
    }

    pub fn walk(mut self) -> io::Result<Self> {
        self.collect(self.root)?;
        Ok(self)
    }

    pub fn sources(&self) -> &[PathBuf] {
        &self.sources
    }

    pub fn hidden_folders(&self) -> Vec<String> {
        self.hidden_folders
            .iter()
            .filter_map(|folder| ModuleNames::of_folder(folder.strip_prefix(self.root).ok()?))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn project_files(&self) -> Vec<String> {
        self.project_files
            .iter()
            .filter_map(|file| ModuleNames::of_source(file.strip_prefix(self.root).ok()?))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn collect(&mut self, directory: &Path) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                self.collect_folder(path)?;
            } else if ProgramFile::is_project_file(&path) {
                self.project_files.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == Language::EXTENSION)
            {
                self.sources.push(path);
            }
        }
        Ok(())
    }

    fn collect_folder(&mut self, folder: PathBuf) -> io::Result<()> {
        match self.skipped.skip_reason(&folder) {
            Some(SkipReason::HiddenFolder) if !folder.is_symlink() => {
                self.hidden_folders.push(folder)
            }
            Some(SkipReason::HiddenFolder) => {}
            Some(SkipReason::BuildFolder) => {}
            None if self.leaves_the_tree(&folder) => {}
            None => self.collect(&folder)?,
        }
        Ok(())
    }

    fn leaves_the_tree(&self, folder: &Path) -> bool {
        self.bounds
            .as_ref()
            .is_some_and(|bounds| bounds.inside(folder).is_none())
    }
}
