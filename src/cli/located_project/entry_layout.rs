use std::path::Path;

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
        self.leaves_out_entry() && self.has_tasks
    }

    fn leaves_out_entry(&self) -> bool {
        let entry = &self.modules.entry;
        !entry.is_written() && !self.root.join(entry.path()).exists()
    }
}
