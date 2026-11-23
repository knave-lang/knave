use std::path::Path;

use super::file_placement::FilePlacement;
use crate::project::Project;

#[derive(Clone, Copy)]
pub struct TaskLayout {
    exports_tasks: bool,
    tasks_folder: bool,
    entry_left_out: bool,
}

impl TaskLayout {
    pub fn of(project: &Project, placement: &FilePlacement<'_>, root: &Path) -> Self {
        let entry = &project.modules.entry;
        Self {
            exports_tasks: project.exported_tasks.is_written(),
            tasks_folder: placement.tasks_directory().is_dir(),
            entry_left_out: !entry.is_written() && !root.join(entry.path()).exists(),
        }
    }

    pub fn exports_tasks(self) -> bool {
        self.exports_tasks
    }

    pub fn has_tasks(self) -> bool {
        self.tasks_folder || self.exports_tasks
    }

    pub fn checks_tasks_alone(self) -> bool {
        self.entry_left_out && self.has_tasks()
    }
}
