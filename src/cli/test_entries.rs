use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::standard_error::StandardError;
use super::terminal::{ModuleOfSources, Terminal};
use crate::language::Language;

pub struct TestEntries {
    path: Option<PathBuf>,
}

pub struct EntryTree {
    path: PathBuf,
    project: Option<LocatedProject>,
    skipped: SkippedDirectory,
    rooting: Rooting,
}

#[derive(Clone, Copy)]
enum Rooting {
    Tests,
    Tasks,
}

impl TestEntries {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub fn load(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let tree = match &self.path {
            Some(path) => EntryTree::at(path.clone(), terminal)?,
            None => EntryTree::of_working_project(terminal)?,
        };
        tree.refuse_crates(terminal)?;
        tree.load(terminal)
    }
}

impl Rooting {
    fn entry_name(self) -> &'static str {
        match self {
            Self::Tests => EntryTree::TESTS_ENTRY,
            Self::Tasks => EntryTree::TASKS_ENTRY,
        }
    }

    fn shares_source_paths(self) -> bool {
        matches!(self, Self::Tests)
    }
}

impl EntryTree {
    const TESTS_ENTRY: &'static str = "<tests>";
    const TASKS_ENTRY: &'static str = "<tasks>";
    const COMMAND: &'static str = "test";

    fn refuse_crates(&self, terminal: &Terminal) -> Result<(), ExitStatus> {
        match &self.project {
            Some(project) => project.refuse_crates(Self::COMMAND, terminal),
            None => Ok(()),
        }
    }

    fn at(path: PathBuf, terminal: &Terminal) -> Result<Self, ExitStatus> {
        let project = LocatedProject::search_from(&path, terminal)?;
        Ok(Self::with_project(path, project))
    }

    fn of_working_project(terminal: &Terminal) -> Result<Self, ExitStatus> {
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        let project = LocatedProject::search_from(working_directory, terminal)?
            .ok_or_else(LocatedProject::report_no_path)?;
        let path = project.tests_directory();
        if !path.is_dir() {
            return Err(Self::report_no_tests_directory(&project));
        }
        Ok(Self::with_project(path, Some(project)))
    }

    pub fn of_tasks(project: LocatedProject) -> Self {
        Self {
            rooting: Rooting::Tasks,
            ..Self::with_project(project.tasks_directory(), Some(project))
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn project(&self) -> Option<&LocatedProject> {
        self.project.as_ref()
    }

    fn with_project(path: PathBuf, project: Option<LocatedProject>) -> Self {
        let skipped = SkippedDirectory::for_walk_of(&path);
        Self {
            path,
            project,
            skipped,
            rooting: Rooting::Tests,
        }
    }

    pub fn load(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        if self.path.is_dir() {
            self.load_directory(terminal)
        } else {
            Ok(ModuleOfSources {
                file: self.entry_file(),
                sources: vec![terminal.load(&self.path)?],
            })
        }
    }

    fn entry_file(&self) -> ProgramFile {
        match &self.project {
            Some(project) => project.test_file(project.file_within(self.path.clone())),
            None => ProgramFile::beside_itself(self.path.clone()),
        }
    }

    pub fn module_paths(&self) -> Result<Vec<String>, ExitStatus> {
        let mut modules = Vec::new();
        self.collect_modules(&self.path, &mut modules)
            .map_err(|error| Terminal::report_unreadable(&self.path, &error))?;
        modules.sort();
        Ok(modules)
    }

    fn load_directory(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let modules = self.module_paths()?;
        let uses: Vec<String> = modules
            .iter()
            .map(|module| format!("(use {module})"))
            .collect();
        let source = terminal.add_named_source(self.rooting.entry_name(), &uses.join("\n"));
        Ok(ModuleOfSources {
            file: self.directory_file(),
            sources: vec![source],
        })
    }

    fn directory_file(&self) -> ProgramFile {
        let file =
            ProgramFile::within(self.path.join(self.rooting.entry_name()), self.path.clone());
        match (&self.project, self.rooting) {
            (Some(project), Rooting::Tests) => project.test_file(file),
            (Some(project), Rooting::Tasks) => project.task_file(file),
            (None, _) => file,
        }
    }

    fn collect_modules(&self, directory: &Path, modules: &mut Vec<String>) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                self.collect_subdirectory(&path, modules)?;
            } else if let Some(module) = self.module_path(&path) {
                modules.push(module);
            }
        }
        Ok(())
    }

    fn collect_subdirectory(&self, directory: &Path, modules: &mut Vec<String>) -> io::Result<()> {
        if self.skipped.is_skipped(directory) {
            return Ok(());
        }
        self.collect_modules(directory, modules)
    }

    fn module_path(&self, file: &Path) -> Option<String> {
        if file.extension()? != Language::EXTENSION || ProgramFile::is_project_file(file) {
            return None;
        }
        self.source_module_path(file)
            .or_else(|| ProgramFile::module_name(file.strip_prefix(&self.path).ok()?))
    }

    fn source_module_path(&self, file: &Path) -> Option<String> {
        let project = self
            .project
            .as_ref()
            .filter(|_| self.rooting.shares_source_paths())?;
        ProgramFile::relative_module(file, &project.source_root())
    }

    fn report_no_tests_directory(project: &LocatedProject) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no `{}/` directory at the project root\n\
             help: put the project's tests in `{}`, or name a test file or directory",
            LocatedProject::TESTS_DIRECTORY,
            project.tests_directory().display()
        ));
        ExitStatus::UsageError
    }
}
