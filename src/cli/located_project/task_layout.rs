use std::path::Path;

use super::file_placement::FilePlacement;
use crate::cli::lending_libraries::LendingLibraries;
use crate::project::Project;

pub struct TaskLayout<'p> {
    exports_tasks: bool,
    tasks_folder: bool,
    entry_left_out: bool,
    lending: &'p LendingLibraries,
}

pub struct ProjectFacts<'p> {
    pub project: &'p Project,
    pub root: &'p Path,
    pub lending: &'p LendingLibraries,
}

impl<'p> TaskLayout<'p> {
    pub fn of(facts: ProjectFacts<'p>, placement: &FilePlacement<'_>) -> Self {
        let entry = &facts.project.modules.entry;
        Self {
            exports_tasks: facts.project.exported_tasks.is_written(),
            tasks_folder: placement.tasks_directory().is_dir(),
            entry_left_out: !entry.is_written() && !facts.root.join(entry.path()).exists(),
            lending: facts.lending,
        }
    }

    pub fn lending(&self) -> &'p LendingLibraries {
        self.lending
    }

    pub fn exports_tasks(&self) -> bool {
        self.exports_tasks
    }

    pub fn has_tasks(&self) -> bool {
        self.tasks_folder || self.exports_tasks
    }

    pub fn checks_tasks_alone(&self) -> bool {
        self.entry_left_out && self.has_tasks()
    }
}
