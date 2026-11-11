use std::path::Path;
use std::rc::Rc;

use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::project_task::{ProjectTask, TaskName};
use super::standard_error::StandardError;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use crate::build::BuildProfile;
use crate::diagnostic::SimilarName;
use crate::language::Language;
use crate::modules::SearchScope;

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
        terminal.report_diagnostics_to(TaskCatalog::user_output(terminal.diagnostic_output()));
        match self.execute(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn execute(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let tree = Self::task_tree(terminal)?;
        let project = tree.project().expect("the task tree belongs to a project");
        let catalog = TaskCatalog::load(&tree, terminal)?;
        let Some(name) = &self.name else {
            let listing = catalog.listing(terminal)?;
            return Ok(terminal.write_output(listing));
        };
        let names: Vec<Rc<str>> = catalog
            .names(terminal)?
            .iter()
            .map(|known| Rc::from(known.as_str()))
            .collect();
        Self::expect_task(&names, name)?;
        let task = ProjectTask::new(project, TaskName::new(name.clone()));
        let binary = TaskRunners::new(&catalog).build(terminal, &task, self.profile)?;
        Ok(FinishedProgram::run_in(&binary, &self.arguments, task.root()).status())
    }

    fn task_tree(terminal: &Terminal) -> Result<EntryTree, ExitStatus> {
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        let project =
            LocatedProject::search_from(working_directory, terminal, FetchingStore::default())?
                .ok_or_else(Self::report_no_project)?;
        if !project.placement().tasks_directory().is_dir() {
            return Err(Self::report_no_tasks_directory());
        }
        Ok(EntryTree::of_tasks(project))
    }

    fn expect_task(names: &[Rc<str>], name: &str) -> Result<(), ExitStatus> {
        if names.iter().any(|known| known.as_ref() == name) {
            return Ok(());
        }
        let help = match SimilarName::closest(name, names) {
            Some(closest) => format!("did you mean `{closest}`?"),
            None if names.is_empty() => {
                "no `deftask` in the tasks directory defines a task yet".to_string()
            }
            None => format!("run `{} {}` to list the tasks", Language::NAME, Self::NAME),
        };
        StandardError::write_line(format_args!("error: no task named `{name}`\nhelp: {help}"));
        Err(ExitStatus::UsageError)
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
             help: define a task with `deftask` in a file under `{0}/`",
            SearchScope::TASKS_DIRECTORY
        ));
        ExitStatus::UsageError
    }
}
