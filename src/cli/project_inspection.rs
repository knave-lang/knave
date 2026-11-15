use super::entry_argument::EntryArgument;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::source_report::SourceReport;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use crate::diagnostic::Diagnostics;

pub type WarningPrinter = dyn Fn(&SourceReport<'_>, &Diagnostics) -> ExitStatus;

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
        print_warnings: &WarningPrinter,
        inspect_entry: impl FnOnce(
            &mut Terminal,
            &ProgramFile,
            &WarningPrinter,
        ) -> Result<(), ExitStatus>,
    ) -> Result<(), ExitStatus> {
        let checks_tasks = self.entry.defaults_to_project_entry();
        let program = self.entry.locate(terminal)?;
        let output = TaskCatalog::user_output(terminal.diagnostic_output());
        terminal.configure(|settings| settings.rerouted_to(output));
        let entry = inspect_entry(terminal, program.file(), print_warnings);
        let tasks = match program.into_project().filter(|_| checks_tasks) {
            Some(project) => {
                Self::check_tasks(terminal, EntryTree::of_tasks(project), print_warnings)
            }
            None => Ok(()),
        };
        entry.and(tasks)
    }

    fn check_tasks(
        terminal: &mut Terminal,
        tree: EntryTree,
        print_warnings: &WarningPrinter,
    ) -> Result<(), ExitStatus> {
        if !tree.path().is_dir() {
            return Ok(());
        }
        let catalog = TaskCatalog::load(&tree, terminal)?;
        let warnings = TaskRunners::new(&catalog).check(terminal)?;
        print_warnings(&terminal.source_report(), &warnings).passed()
    }
}
