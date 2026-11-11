use super::entry_argument::EntryArgument;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;

pub struct CheckCommand {
    entry: EntryArgument,
}

impl CheckCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let checks_tasks = self.entry.defaults_to_project_entry();
        let program = match self.entry.locate(terminal) {
            Ok(program) => program,
            Err(status) => return status,
        };
        terminal.report_diagnostics_to(TaskCatalog::user_output(terminal.diagnostic_output()));
        let entry = terminal.check_file(program.file()).map(|_| ());
        let tasks = match program.into_project().filter(|_| checks_tasks) {
            Some(project) => Self::check_tasks(terminal, EntryTree::of_tasks(project)),
            None => Ok(()),
        };
        match entry.and(tasks) {
            Ok(()) => ExitStatus::Success,
            Err(status) => status,
        }
    }

    fn check_tasks(terminal: &mut Terminal, tree: EntryTree) -> Result<(), ExitStatus> {
        if !tree.path().is_dir() {
            return Ok(());
        }
        let catalog = TaskCatalog::load(&tree, terminal)?;
        TaskRunners::new(&catalog).check(terminal)
    }
}
