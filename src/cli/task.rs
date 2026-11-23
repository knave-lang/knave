use std::rc::Rc;

use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::library_task_run::LibraryTaskRun;
use super::library_tasks::LibraryTasks;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::own_tasks::OwnTasks;
use super::separated_batches::SeparatedBatches;
use super::standard_error::StandardError;
use super::standard_output::StandardOutput;
use super::task_address::TaskAddress;
use super::task_catalog::TaskCatalog;
use super::task_invocation::TaskInvocation;
use super::task_name_lookup::TaskNameLookup;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::build::BuildProfile;
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
        match self.name.as_deref().map(TaskAddress::parse).transpose()? {
            Some(address) => self.run_address(terminal, project, address),
            None => {
                let own = OwnTasks::load(terminal, project)?;
                Self::list(terminal, &own)
            }
        }
    }

    fn expect_tasks(project: &LocatedProject) -> Result<(), ExitStatus> {
        let layout = project.task_layout();
        if layout.has_tasks() || !layout.lending().lenders().is_empty() {
            return Ok(());
        }
        Err(Self::report_no_tasks_directory())
    }

    fn run_address(
        &self,
        terminal: &mut Terminal,
        project: LocatedProject,
        address: TaskAddress<'_>,
    ) -> Result<ExitStatus, ExitStatus> {
        match address {
            TaskAddress::Qualified { library, task } => {
                let lending = project.task_layout().lending();
                let library = TaskNameLookup::new(lending, Self::NAME).qualified(library, task)?;
                LibraryTaskRun::new(&project, library, self.invocation()).run(terminal, task)
            }
            TaskAddress::Bare(task) => {
                let own = OwnTasks::load(terminal, project)?;
                self.run_bare(terminal, &own, task)
            }
        }
    }

    fn run_bare(
        &self,
        terminal: &mut Terminal,
        own: &OwnTasks,
        task: &str,
    ) -> Result<ExitStatus, ExitStatus> {
        if own.defines(task) {
            return own.run(terminal, task, self.invocation());
        }
        let lending = own.project().task_layout().lending();
        let library = TaskNameLookup::new(lending, Self::NAME).bare(task, own.names())?;
        LibraryTaskRun::new(own.project(), library, self.invocation()).run(terminal, task)
    }

    fn invocation(&self) -> TaskInvocation<'_> {
        TaskInvocation {
            arguments: &self.arguments,
            profile: self.profile,
        }
    }

    fn list(terminal: &mut Terminal, own: &OwnTasks) -> Result<ExitStatus, ExitStatus> {
        let lending = own.project().task_layout().lending();
        let mut listing = own.listing(terminal)?;
        let mut failure = Ok(());
        for library in lending.lenders() {
            let arguments = lending.listing_arguments(library, own.names());
            match LibraryTasks::within(terminal, library, |terminal, tasks| {
                tasks.listing(terminal, &arguments)
            }) {
                Ok(block) => Self::append_block(&mut listing, &block),
                Err(status) => failure = Err(status),
            }
        }
        let written = StandardOutput::locked().write(listing);
        failure.map(|()| written)
    }

    fn append_block(listing: &mut String, block: &str) {
        if !listing.is_empty() {
            listing.push('\n');
        }
        listing.push_str(block);
    }

    fn project(terminal: &mut Terminal) -> Result<LocatedProject, ExitStatus> {
        let working_directory = WorkingDirectory::path();
        LocatedProject::enter_from(working_directory, terminal, FetchingStore::default())?
            .ok_or_else(Self::report_no_project)
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
