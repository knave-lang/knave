use super::file_placement::FilePlacement;
use crate::cli::lending_libraries::LendingLibraries;
use crate::project::Project;

pub struct TaskLayout<'p> {
    exports_tasks: bool,
    tasks_folder: bool,
    lending: &'p LendingLibraries,
}

pub struct ProjectFacts<'p> {
    pub project: &'p Project,
    pub lending: &'p LendingLibraries,
}

impl<'p> TaskLayout<'p> {
    pub fn of(facts: ProjectFacts<'p>, placement: &FilePlacement<'_>) -> Self {
        Self {
            exports_tasks: facts.project.exported_tasks.is_written(),
            tasks_folder: placement.tasks_directory().is_dir(),
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
}
