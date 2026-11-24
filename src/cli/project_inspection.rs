use std::rc::Rc;

use super::entry_argument::EntryArgument;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::library_tasks::LibraryTasks;
use super::located_program::LocatedProgram;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::separated_batches::SeparatedBatches;
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
        inspect_entry: impl Fn(&mut Terminal, &ProgramFile, &WarningPrinter) -> Result<(), ExitStatus>,
    ) -> Result<(), ExitStatus> {
        let defaults_to_project_entry = self.entry.defaults_to_project_entry();
        let program = self.entry.locate(terminal)?;
        let checks_tasks = defaults_to_project_entry || Self::names_a_task_file(&program);
        let skips_entry = defaults_to_project_entry
            && program
                .project()
                .is_some_and(|project| project.entry_layout().skips_entry());
        let separated = Rc::new(SeparatedBatches::new(terminal.diagnostic_output()));
        let output = TaskCatalog::user_output(separated);
        terminal.configure(|settings| settings.rerouted_to(output));
        let entry = if skips_entry {
            Ok(())
        } else {
            inspect_entry(terminal, program.file(), print_warnings)
        };
        let inspected_project = program.project().filter(|_| defaults_to_project_entry);
        let library = match Self::unchecked_library_root(terminal, inspected_project) {
            Some(library_root) => inspect_entry(terminal, &library_root, print_warnings),
            None => Ok(()),
        };
        let Some(project) = program.into_project().filter(|_| checks_tasks) else {
            return entry.and(library);
        };
        let tree = EntryTree::of_tasks(project);
        let tasks = Self::inspect_tasks(terminal, &tree, print_warnings);
        let lent = Self::check_lenders(
            terminal,
            tree.project().filter(|_| defaults_to_project_entry),
        );
        entry.and(library).and(tasks).and(lent)
    }

    fn unchecked_library_root(
        terminal: &Terminal,
        project: Option<&LocatedProject>,
    ) -> Option<ProgramFile> {
        let layout = project?.entry_layout();
        let checked = terminal.source_report().canonical_files_from(0);
        layout.library_root_outside(&checked)
    }

    fn inspect_tasks(
        terminal: &mut Terminal,
        tree: &EntryTree,
        print_warnings: &WarningPrinter,
    ) -> Result<(), ExitStatus> {
        if tree.holds_tasks() {
            Self::check_tasks(terminal, tree, print_warnings)
        } else {
            Ok(())
        }
    }

    fn check_lenders(
        terminal: &mut Terminal,
        project: Option<&LocatedProject>,
    ) -> Result<(), ExitStatus> {
        match project {
            Some(project) => LibraryTasks::check_lenders(terminal, project.task_layout().lending()),
            None => Ok(()),
        }
    }

    fn names_a_task_file(program: &LocatedProgram) -> bool {
        let path = program.file().path();
        program
            .project()
            .is_some_and(|project| project.placement().trees().holds_task(path))
    }

    fn check_tasks(
        terminal: &mut Terminal,
        tree: &EntryTree,
        print_warnings: &WarningPrinter,
    ) -> Result<(), ExitStatus> {
        let catalog = TaskCatalog::load(tree, terminal, "")?;
        let warnings = TaskRunners::new(&catalog).check(terminal)?;
        print_warnings(&terminal.source_report(), &warnings).passed()
    }
}
