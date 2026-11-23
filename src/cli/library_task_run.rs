use std::rc::Rc;

use super::direct_library::DirectLibrary;
use super::entry_tree::EntryTree;
use super::errors_only_output::ErrorsOnlyOutput;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::library_task::{LibraryTask, LibraryTaskSite};
use super::library_walk::FetchingStore;
use super::lint_levels::LintLevels;
use super::located_project::LocatedProject;
use super::task_catalog::TaskCatalog;
use super::task_invocation::TaskInvocation;
use super::task_name::TaskName;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;

pub struct LibraryTaskRun<'r> {
    dependent: &'r LocatedProject,
    library: &'r DirectLibrary,
    invocation: TaskInvocation<'r>,
}

impl<'r> LibraryTaskRun<'r> {
    pub fn new(
        dependent: &'r LocatedProject,
        library: &'r DirectLibrary,
        invocation: TaskInvocation<'r>,
    ) -> Self {
        Self {
            dependent,
            library,
            invocation,
        }
    }

    pub fn run(&self, terminal: &mut Terminal, task: &str) -> Result<ExitStatus, ExitStatus> {
        terminal.configure(|settings| {
            let errors = Rc::new(ErrorsOnlyOutput::new(Rc::clone(settings.diagnostics())));
            settings.rerouted_to(errors)
        });
        let tree = EntryTree::of_tasks(self.library_project(terminal)?);
        let catalog = TaskCatalog::load(&tree, terminal, Some(self.library))?;
        catalog.names(terminal)?;
        catalog.exports()?;
        let library = tree
            .project()
            .expect("a library's task tree keeps its project");
        let site = LibraryTaskSite {
            dependent: self.dependent,
            library,
            library_name: &self.library.name,
        };
        let target = LibraryTask::new(&site, TaskName::new(task.to_string()));
        target
            .copy_lock_from(library)
            .map_err(|error| Self::report_lock(terminal, &error))?;
        let binary =
            TaskRunners::new(&catalog).build(terminal, &target, self.invocation.profile)?;
        Ok(
            FinishedProgram::run_in(&binary, self.invocation.arguments, self.dependent.root())
                .status(),
        )
    }

    fn library_project(&self, terminal: &mut Terminal) -> Result<LocatedProject, ExitStatus> {
        let library = LocatedProject::enter_unchecked(
            &self.library.root,
            terminal,
            FetchingStore::default(),
        )?
        .expect("a listed library has a project file");
        terminal.configure(|settings| settings.leveling_lints_by(LintLevels::default()));
        Ok(library)
    }

    fn report_lock(terminal: &Terminal, error: &std::io::Error) -> ExitStatus {
        terminal
            .diagnostic_output()
            .write_error(&format!("cannot copy the library's `Cargo.lock`: {error}"));
        ExitStatus::Environment
    }
}
