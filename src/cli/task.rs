use std::path::Path;
use std::rc::Rc;

use super::distinct_output::DistinctOutput;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::task_lookup::TaskLookup;
use super::terminal::Terminal;
use super::test_entries::EntryTree;
use crate::build::BuildProfile;
use crate::diagnostic::SimilarName;
use crate::language::Language;

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
        let distinct = DistinctOutput::new(terminal.diagnostic_output());
        terminal.report_diagnostics_to(Rc::new(distinct));
        match self.execute(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn execute(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let tree = Self::task_tree(terminal)?;
        let project = tree.project().expect("the task tree belongs to a project");
        let lookup = TaskLookup::load(&tree, terminal)?;
        let Some(name) = &self.name else {
            let listing = lookup.listing(terminal)?;
            return Ok(terminal.write_output(listing));
        };
        let names: Vec<Rc<str>> = lookup.names(terminal)?.into_iter().map(Rc::from).collect();
        Self::expect_task(&names, name)?;
        let binary = lookup.build(terminal, project, name, self.profile)?;
        Ok(FinishedProgram::run_in(&binary, &self.arguments, project.root()).status())
    }

    fn task_tree(terminal: &Terminal) -> Result<EntryTree, ExitStatus> {
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        let project = LocatedProject::search_from(working_directory, terminal)?
            .ok_or_else(Self::report_no_project)?;
        let directory = project.tasks_directory();
        if !directory.is_dir() {
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
            LocatedProject::TASKS_DIRECTORY
        ));
        ExitStatus::UsageError
    }
}
