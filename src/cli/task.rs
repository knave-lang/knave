use std::rc::Rc;

use super::embedded_prelude::EmbeddedPrelude;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::library_task_resolver::LibraryTaskResolver;
use super::library_task_run::LibraryTaskRun;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::own_tasks::OwnTasks;
use super::separated_batches::SeparatedBatches;
use super::standard_error::StandardError;
use super::standard_output::StandardOutput;
use super::task_address::TaskAddress;
use super::task_catalog::TaskCatalog;
use super::task_invocation::TaskInvocation;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::build::BuildProfile;
use crate::diagnostic::SimilarName;
use crate::language::Language;
use crate::modules::SearchScope;
use crate::project::Section;

pub struct TaskCommand {
    name: Option<String>,
    arguments: Vec<String>,
    profile: BuildProfile,
}

impl TaskCommand {
    pub const NAME: &'static str = "task";

    pub fn new(command: Vec<String>, profile: BuildProfile) -> Self {
        let mut words = command.into_iter();
        Self {
            name: words.next(),
            arguments: words.collect(),
            profile,
        }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let separated = Rc::new(SeparatedBatches::new(terminal.diagnostic_output()));
        let output = TaskCatalog::user_output(separated);
        terminal.configure(|settings| settings.rerouted_to(output));
        match self.execute(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn execute(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let project = Self::project(terminal)?;
        Self::expect_tasks(&project)?;
        let Some(written) = &self.name else {
            return Self::list(terminal, project);
        };
        let address = TaskAddress::parse(written)?;
        self.run_address(terminal, project, address)
    }

    fn run_address(
        &self,
        terminal: &mut Terminal,
        project: LocatedProject,
        address: TaskAddress<'_>,
    ) -> Result<ExitStatus, ExitStatus> {
        let libraries = project.task_layout().libraries().to_vec();
        let resolver = LibraryTaskResolver::new(&libraries);
        match address {
            TaskAddress::Qualified { library, task } => {
                let library = resolver.qualified(library, task)?;
                LibraryTaskRun::new(&project, library, self.invocation()).run(terminal, task)
            }
            TaskAddress::Bare(task) => {
                let own = OwnTasks::load(terminal, project)?;
                if let Some(outcome) = own.run(terminal, task, self.invocation()) {
                    return outcome;
                }
                if !resolver.has_exports() {
                    return Err(Self::report_unknown(own.names(), task));
                }
                let library = resolver.bare(task, own.names())?;
                LibraryTaskRun::new(own.project(), library, self.invocation()).run(terminal, task)
            }
        }
    }

    fn invocation(&self) -> TaskInvocation<'_> {
        TaskInvocation {
            arguments: &self.arguments,
            profile: self.profile,
        }
    }

    fn expect_tasks(project: &LocatedProject) -> Result<(), ExitStatus> {
        let layout = project.task_layout();
        let lent = layout
            .libraries()
            .iter()
            .any(|library| !library.exported_tasks.is_empty());
        if layout.has_tasks() || lent {
            return Ok(());
        }
        Err(Self::report_no_tasks_directory())
    }

    fn list(terminal: &mut Terminal, project: LocatedProject) -> Result<ExitStatus, ExitStatus> {
        if !project.task_layout().has_tasks() {
            return Ok(ExitStatus::Success);
        }
        let tree = EntryTree::of_tasks(project);
        let catalog = TaskCatalog::load(&tree, terminal, None)?;
        let listing = catalog.listing(terminal)?;
        Ok(StandardOutput::locked().write(listing))
    }

    fn project(terminal: &mut Terminal) -> Result<LocatedProject, ExitStatus> {
        let working_directory = WorkingDirectory::path();
        LocatedProject::enter_from(working_directory, terminal, FetchingStore::default())?
            .ok_or_else(Self::report_no_project)
    }

    fn report_unknown(names: &[Rc<str>], name: &str) -> ExitStatus {
        let help = match SimilarName::closest(name, names) {
            Some(closest) => format!("did you mean `{closest}`?"),
            None if names.is_empty() => {
                "no `deftask` under `tasks/` defines a task yet".to_string()
            }
            None => format!("run `{} {}` to list the tasks", Language::NAME, Self::NAME),
        };
        StandardError::write_line(format_args!("error: no task named `{name}`\nhelp: {help}"));
        ExitStatus::UsageError
    }

    fn report_no_project() -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no `{}` in this directory or a parent\n\
             help: run `{} {}` inside a project",
            Language::project_file_name(),
            Language::NAME,
            Self::NAME
        ));
        ExitStatus::UsageError
    }

    fn report_no_tasks_directory() -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no `{0}/` directory at the project root\n\
             help: define a task with `deftask` in a file under `{0}/`, \
             or list a library whose `{1}` section names one",
            SearchScope::TASKS_DIRECTORY,
            EmbeddedPrelude::project_vocabulary().section_name(Section::ExportedTasks)
        ));
        ExitStatus::UsageError
    }
}
