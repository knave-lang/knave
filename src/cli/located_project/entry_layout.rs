use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use super::file_placement::FilePlacement;
use crate::cli::program_file::ProgramFile;
use crate::project::ProjectModules;

pub struct EntryLayout<'p> {
    root: &'p Path,
    modules: &'p ProjectModules,
    placement: FilePlacement<'p>,
    has_tasks: bool,
}

pub struct EntryFacts<'p> {
    pub root: &'p Path,
    pub modules: &'p ProjectModules,
    pub has_tasks: bool,
}

impl<'p> EntryLayout<'p> {
    pub fn of(facts: EntryFacts<'p>, placement: FilePlacement<'p>) -> Self {
        Self {
            root: facts.root,
            modules: facts.modules,
            placement,
            has_tasks: facts.has_tasks,
        }
    }

    pub fn entry(&self) -> ProgramFile {
        let entry = self.root.join(self.modules.entry.path());
        self.placement.file_within(entry)
    }

    pub fn is_entry(&self, file: &ProgramFile) -> bool {
        let entry = self.root.join(self.modules.entry.path());
        ProgramFile::canonical(&entry) == ProgramFile::canonical(file.path())
    }

    pub fn skips_entry(&self) -> bool {
        self.leaves_out_entry() && self.starts_elsewhere()
    }

    pub fn library_root_outside(&self, loaded: &FxHashSet<PathBuf>) -> Option<ProgramFile> {
        self.library_root().filter(|library_root| {
            ProgramFile::canonical(library_root.path()).is_some_and(|path| !loaded.contains(&path))
        })
    }

    fn leaves_out_entry(&self) -> bool {
        let entry = &self.modules.entry;
        !entry.is_written() && !self.root.join(entry.path()).exists()
    }

    fn starts_elsewhere(&self) -> bool {
        self.has_tasks || self.library_root().is_some()
    }

    fn library_root(&self) -> Option<ProgramFile> {
        let library_root = self.root.join(&self.modules.library_root);
        let file = self.placement.file_within(library_root);
        (file.path().is_file() && !self.is_entry(&file)).then_some(file)
    }
}
