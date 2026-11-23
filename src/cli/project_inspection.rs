use std::cell::Cell;
use std::rc::Rc;

use super::diagnostic_output::{DiagnosticOutput, MessageOutput};
use super::entry_argument::EntryArgument;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::located_program::LocatedProgram;
use super::located_project::TaskLayout;
use super::program_file::ProgramFile;
use super::source_report::SourceReport;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

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
        let defaults_to_project_entry = self.entry.defaults_to_project_entry();
        let program = self.entry.locate(terminal)?;
        let checks_tasks = defaults_to_project_entry || Self::names_a_task_file(&program);
        let skips_entry = defaults_to_project_entry
            && program
                .project()
                .is_some_and(|project| Self::checks_tasks_alone(project.task_layout()));
        let separated = Rc::new(SeparatedBatches::new(terminal.diagnostic_output()));
        let output = TaskCatalog::user_output(separated);
        terminal.configure(|settings| settings.rerouted_to(output));
        let entry = match skips_entry {
            true => Ok(()),
            false => inspect_entry(terminal, program.file(), print_warnings),
        };
        let tasks = match program.into_project().filter(|_| checks_tasks) {
            Some(project) => {
                Self::check_tasks(terminal, EntryTree::of_tasks(project), print_warnings)
            }
            None => Ok(()),
        };
        entry.and(tasks)
    }

    fn checks_tasks_alone(layout: TaskLayout) -> bool {
        layout.entry_left_out && (layout.tasks_folder || layout.exports_tasks)
    }

    fn names_a_task_file(program: &LocatedProgram) -> bool {
        let path = program.file().path();
        program
            .project()
            .is_some_and(|project| project.placement().trees().holds_task(path))
    }

    fn check_tasks(
        terminal: &mut Terminal,
        tree: EntryTree,
        print_warnings: &WarningPrinter,
    ) -> Result<(), ExitStatus> {
        if !tree.holds_tasks() {
            return Ok(());
        }
        let catalog = TaskCatalog::load(&tree, terminal)?;
        let warnings = TaskRunners::new(&catalog).check(terminal)?;
        print_warnings(&terminal.source_report(), &warnings).passed()
    }
}

struct SeparatedBatches {
    inner: Rc<dyn DiagnosticOutput>,
    wrote_a_batch: Cell<bool>,
}

impl SeparatedBatches {
    fn new(inner: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            inner,
            wrote_a_batch: Cell::new(false),
        }
    }
}

impl MessageOutput for SeparatedBatches {}

impl DiagnosticOutput for SeparatedBatches {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        match self.wrote_a_batch.replace(true) {
            true => self.inner.write_after_earlier(sources, diagnostics),
            false => self.inner.write(sources, diagnostics),
        }
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.inner.write_note(sources, message);
    }
}
