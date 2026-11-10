use std::path::{Path, PathBuf};

use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::terminal::{ModuleOfSources, Terminal};
use crate::modules::SearchScope;

pub struct TestEntries {
    path: Option<PathBuf>,
}

impl TestEntries {
    const COMMAND: &'static str = "test";

    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub fn load(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let tree = match &self.path {
            Some(path) => Self::at(path.clone(), terminal)?,
            None => Self::of_working_project(terminal)?,
        };
        Self::refuse_crates(&tree, terminal)?;
        tree.load(terminal)
    }

    fn refuse_crates(tree: &EntryTree, terminal: &Terminal) -> Result<(), ExitStatus> {
        match tree.project() {
            Some(project) => project.refuse_crates(Self::COMMAND, terminal),
            None => Ok(()),
        }
    }

    fn at(path: PathBuf, terminal: &Terminal) -> Result<EntryTree, ExitStatus> {
        let project = LocatedProject::search_from(&path, terminal)?;
        Ok(EntryTree::of_tests(path, project))
    }

    fn of_working_project(terminal: &Terminal) -> Result<EntryTree, ExitStatus> {
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        let project = LocatedProject::search_from(working_directory, terminal)?
            .ok_or_else(LocatedProject::report_no_path)?;
        let path = project.tests_directory();
        if !path.is_dir() {
            return Err(Self::report_no_tests_directory(&project));
        }
        Ok(EntryTree::of_tests(path, Some(project)))
    }

    fn report_no_tests_directory(project: &LocatedProject) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no `{}/` directory at the project root\n\
             help: put the project's tests in `{}`, or name a test file or directory",
            SearchScope::TESTS_DIRECTORY,
            project.tests_directory().display()
        ));
        ExitStatus::UsageError
    }
}
