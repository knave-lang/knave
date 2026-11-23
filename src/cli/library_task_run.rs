use super::direct_library::DirectLibrary;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::library_task::{LibraryTask, LibraryTaskSite};
use super::library_tasks::LibraryTasks;
use super::located_project::LocatedProject;
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
        let binary = LibraryTasks::within(terminal, self.library, |terminal, tasks| {
            let LibraryTasks {
                tree,
                catalog,
                exports,
            } = tasks;
            exports.result()?;
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
            TaskRunners::new(&catalog).build(terminal, &target, self.invocation.profile)
        })?;
        Ok(
            FinishedProgram::run_in(&binary, self.invocation.arguments, self.dependent.root())
                .status(),
        )
    }

    fn report_lock(terminal: &Terminal, error: &std::io::Error) -> ExitStatus {
        terminal
            .diagnostic_output()
            .write_error(&format!("cannot copy the library's `Cargo.lock`: {error}"));
        ExitStatus::Environment
    }
}
