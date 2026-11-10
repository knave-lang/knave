use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::standard_error::StandardError;
use super::terminal::{ModuleOfSources, Terminal};
use super::tree_bounds::TreeBounds;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::ExpandProblems;
use crate::language::Language;
use crate::modules::ModuleSources;
use crate::source::Span;

pub struct EntryTree {
    path: PathBuf,
    project: Option<LocatedProject>,
    skipped: SkippedDirectory,
    kind: TreeKind,
}

#[derive(Clone, Copy)]
enum TreeKind {
    Tests,
    Tasks,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeFile {
    module: String,
    path: PathBuf,
}

impl TreeFile {
    const UNNAMABLE: [char; 4] = ['(', ')', '"', ';'];

    pub fn new(module: String, path: PathBuf) -> Self {
        Self { module, path }
    }

    pub fn into_module(self) -> String {
        self.module
    }

    pub fn is_unnamable(&self) -> bool {
        self.module
            .chars()
            .any(|character| character.is_whitespace() || Self::UNNAMABLE.contains(&character))
    }

    pub fn report_unnamable(&self) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{}` cannot be a module, because a module path cannot hold a space, a parenthesis, a quote or a semicolon\n\
             help: rename the file",
            self.path.display()
        ));
        ExitStatus::UsageError
    }

    pub fn refuse_compiler_modules(
        files: &[TreeFile],
        terminal: &mut Terminal,
    ) -> Result<(), ExitStatus> {
        let mut problems = Diagnostics::default();
        for file in files.iter().filter(|file| file.takes_compiler_module()) {
            let start = Span::new(terminal.load(&file.path)?, 0..0);
            problems.push(ExpandProblems::file_takes_compiler_module(
                &file.module_path(),
                start,
            ));
        }
        if problems.has_errors() {
            return Err(terminal.report(&problems));
        }
        Ok(())
    }

    fn module_path(&self) -> ModulePath {
        ModulePath::new(self.module.as_str())
    }

    fn takes_compiler_module(&self) -> bool {
        !ModuleSources::is_written(&self.module_path())
    }
}

impl TreeKind {
    const TESTS_ENTRY: &'static str = "<tests>";
    const TASKS_ENTRY: &'static str = "<tasks>";
    fn stays_inside(self) -> bool {
        matches!(self, Self::Tasks)
    }

    fn refuses(self, file: &TreeFile) -> bool {
        self.stays_inside() && file.is_unnamable()
    }

    fn entry_name(self) -> &'static str {
        match self {
            Self::Tests => Self::TESTS_ENTRY,
            Self::Tasks => Self::TASKS_ENTRY,
        }
    }

    fn shares_source_paths(self) -> bool {
        matches!(self, Self::Tests)
    }

    fn program_file(self, project: &LocatedProject, file: ProgramFile) -> ProgramFile {
        match self {
            Self::Tests => project.test_file(file),
            Self::Tasks => project.task_file(file),
        }
    }
}

impl EntryTree {
    pub const TASKS_ENTRY: &'static str = TreeKind::TASKS_ENTRY;

    pub fn of_tests(path: PathBuf, project: Option<LocatedProject>) -> Self {
        let skipped = SkippedDirectory::for_walk_of(&path);
        Self {
            path,
            project,
            skipped,
            kind: TreeKind::Tests,
        }
    }

    pub fn of_tasks(project: LocatedProject) -> Self {
        Self {
            kind: TreeKind::Tasks,
            ..Self::of_tests(project.tasks_directory(), Some(project))
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn project(&self) -> Option<&LocatedProject> {
        self.project.as_ref()
    }

    pub fn into_project(self) -> Option<LocatedProject> {
        self.project
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
        let files = self.module_files()?;
        Ok(files.into_iter().map(TreeFile::into_module).collect())
    }

    fn module_files(&self) -> Result<Vec<TreeFile>, ExitStatus> {
        let mut files = Vec::new();
        self.collect_modules(&self.path, &mut files)
            .map_err(|error| Terminal::report_unreadable(&self.path, &error))?;
        files.sort();
        match files.iter().find(|file| self.kind.refuses(file)) {
            Some(file) => Err(file.report_unnamable()),
            None => Ok(files),
        }
    }

    fn load_directory(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let files = self.module_files()?;
        TreeFile::refuse_compiler_modules(&files, terminal)?;
        let uses: Vec<String> = files
            .iter()
            .map(|file| format!("(use {})", file.module))
            .collect();
        let source = terminal.add_named_source(self.kind.entry_name(), &uses.join("\n"));
        Ok(ModuleOfSources {
            file: self.directory_file(),
            sources: vec![source],
        })
    }

    fn directory_file(&self) -> ProgramFile {
        let file = ProgramFile::within(self.path.join(self.kind.entry_name()), self.path.clone());
        match &self.project {
            Some(project) => self.kind.program_file(project, file),
            None => file,
        }
    }

    fn collect_modules(&self, directory: &Path, files: &mut Vec<TreeFile>) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                self.collect_subdirectory(&path, files)?;
            } else if let Some(module) = self.module_path(&path) {
                files.push(TreeFile::new(module, path));
            }
        }
        Ok(())
    }

    fn collect_subdirectory(&self, directory: &Path, files: &mut Vec<TreeFile>) -> io::Result<()> {
        if self.skipped.is_skipped(directory) || self.leaves_the_tree(directory) {
            return Ok(());
        }
        self.collect_modules(directory, files)
    }

    fn leaves_the_tree(&self, directory: &Path) -> bool {
        self.kind.stays_inside() && TreeBounds::of(&self.path).inside(directory).is_none()
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
            .filter(|_| self.kind.shares_source_paths())?;
        ProgramFile::relative_module(file, &project.source_root())
    }
}
