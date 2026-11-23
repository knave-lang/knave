use super::direct_library::DirectLibrary;
use super::exit_status::ExitStatus;
use super::library_task::LibraryTaskSite;
use super::library_tasks::LibraryTasks;
use super::located_project::LocatedProject;
use super::task_invocation::TaskInvocation;
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
        let site = LibraryTaskSite {
            dependent: self.dependent,
            library_name: self.library.name(),
            task,
        };
        LibraryTasks::within(terminal, self.library, |terminal, tasks| {
            tasks.run(terminal, &site, self.invocation)
        })
    }
}
