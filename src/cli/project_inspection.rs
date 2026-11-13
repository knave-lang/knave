use super::entry_argument::EntryArgument;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use crate::diagnostic::Diagnostics;

pub struct ProjectInspection {
    entry: EntryArgument,
}

impl ProjectInspection {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(
        self,
        terminal: &mut Terminal,
        inspect_entry: impl FnOnce(&mut Terminal, &ProgramFile) -> Result<Diagnostics, ExitStatus>,
    ) -> Result<Diagnostics, ExitStatus> {
        let checks_tasks = self.entry.defaults_to_project_entry();
        let program = self.entry.locate(terminal)?;
        let output = TaskCatalog::user_output(terminal.diagnostic_output());
        terminal.configure(|settings| settings.rerouted_to(output));
        let entry = inspect_entry(terminal, program.file());
        let tasks = match program.into_project().filter(|_| checks_tasks) {
            Some(project) => Self::check_tasks(terminal, EntryTree::of_tasks(project)),
            None => Ok(Diagnostics::default()),
        };
        let (mut printed, task_warnings) = (entry?, tasks?);
        printed.extend(task_warnings);
        Ok(printed)
    }

    fn check_tasks(terminal: &mut Terminal, tree: EntryTree) -> Result<Diagnostics, ExitStatus> {
        if !tree.path().is_dir() {
            return Ok(Diagnostics::default());
        }
        let catalog = TaskCatalog::load(&tree, terminal)?;
        let warnings = TaskRunners::new(&catalog).check(terminal)?;
        terminal.source_report().print(&warnings);
        Ok(warnings)
    }
}
