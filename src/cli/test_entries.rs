use std::path::{Path, PathBuf};

use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::terminal::{ModuleOfSources, Terminal};
use crate::modules::SearchScope;

pub struct TestEntries {
    path: Option<PathBuf>,
}

pub struct LoadedTests {
    pub module: ModuleOfSources,
    pub project: Option<LocatedProject>,
}

impl TestEntries {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub fn load(&self, terminal: &mut Terminal) -> Result<LoadedTests, ExitStatus> {
        let tree = match &self.path {
            Some(path) => Self::at(path.clone(), terminal)?,
            None => Self::of_working_project(terminal)?,
        };
        let module = tree.load(terminal)?;
        Ok(LoadedTests {
            module,
            project: tree.into_project(),
        })
    }

    fn at(path: PathBuf, terminal: &Terminal) -> Result<EntryTree, ExitStatus> {
        let project = LocatedProject::search_from(&path, terminal, FetchingStore::default())?;
        Ok(EntryTree::of_tests(path, project))
    }

    fn of_working_project(terminal: &Terminal) -> Result<EntryTree, ExitStatus> {
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        let project =
            LocatedProject::search_from(working_directory, terminal, FetchingStore::default())?
                .ok_or_else(LocatedProject::report_no_path)?;
        let path = project.placement().tests_directory().to_path_buf();
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
            project.placement().tests_directory().display()
        ));
        ExitStatus::UsageError
    }
}
