use std::path::PathBuf;
use std::rc::Rc;

use super::distinct_output::DistinctOutput;
use super::exit_status::ExitStatus;
use super::located_program::LocatedProgram;
use super::task_lookup::TaskLookup;
use super::terminal::Terminal;
use super::test_entries::EntryTree;

pub struct CheckCommand {
    file: Option<PathBuf>,
}

impl CheckCommand {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self { file }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let checks_tasks = self.file.is_none();
        let program = match LocatedProgram::locate(self.file, terminal) {
            Ok(program) => program,
            Err(status) => return status,
        };
        let distinct = DistinctOutput::new(terminal.diagnostic_output());
        terminal.report_diagnostics_to(Rc::new(distinct));
        let entry = terminal.check(program.file()).map(|_| ());
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
        TaskLookup::load(&tree, terminal)?.check(terminal)
    }
}
