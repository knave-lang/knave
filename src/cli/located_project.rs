use std::path::{Path, PathBuf};

use super::crate_section::CrateSection;
use super::exit_status::ExitStatus;
use super::library_walk::LibraryWalk;
use super::program_file::ProgramFile;
use super::project_crates::ProjectCrates;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::modules::{LibraryTable, SearchScope};
use crate::project::{CrateDependency, Project, ProjectProblems};

pub struct LocatedProject {
    root: PathBuf,
    project: Project,
    libraries: LibraryTable,
    crates: ProjectCrates,
    crate_library: Option<String>,
}

impl LocatedProject {
    pub fn search_from(directory: &Path, terminal: &Terminal) -> Result<Option<Self>, ExitStatus> {
        let Some(root) = Self::nearest_root(directory)? else {
            return Ok(None);
        };
        let mut scratch = terminal.scratch();
        let project = scratch.load_project(&root.join(Language::project_file_name()))?;
        let walked = LibraryWalk::walk(&mut scratch, &root, &project)?;
        let dev_crates = project
            .dev_dependencies
            .iter()
            .map(|dependency| ProgramFile::resolved_crate(dependency, &root))
            .collect();
        let crates = ProjectCrates::new(
            root.clone(),
            project.name.clone(),
            walked.crates,
            dev_crates,
        );
        Ok(Some(Self {
            root,
            project,
            libraries: walked.table,
            crates,
            crate_library: walked.crate_library,
        }))
    }

    fn nearest_root(directory: &Path) -> Result<Option<PathBuf>, ExitStatus> {
        match Self::canonical_root(directory) {
            Some(root) => Ok(Some(Self::shown(&root)?)),
            None => Ok(None),
        }
    }

    pub fn canonical_root(directory: &Path) -> Option<PathBuf> {
        let start = ProgramFile::canonical(directory)?;
        let file_name = Language::project_file_name();
        start
            .ancestors()
            .find(|ancestor| ancestor.join(&file_name).is_file())
            .map(Path::to_path_buf)
    }

    fn shown(root: &Path) -> Result<PathBuf, ExitStatus> {
        let working_directory = Terminal::canonical(Path::new(Terminal::WORKING_DIRECTORY))?;
        Ok(root
            .strip_prefix(&working_directory)
            .unwrap_or(root)
            .to_path_buf())
    }

    pub fn report_no_path() -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no path given, and no `{}` in this directory or a parent\n\
             help: name a path, or run the command inside a project",
            Language::project_file_name()
        ));
        ExitStatus::UsageError
    }

    pub fn lend_libraries(project: Option<&Self>, file: ProgramFile) -> ProgramFile {
        match project {
            Some(project) => file.with_libraries(project.libraries.clone()),
            None => file,
        }
    }

    pub fn name(&self) -> &str {
        &self.project.name
    }

    pub fn entry(&self) -> ProgramFile {
        let entry = self.root.join(&self.project.entry);
        self.file_within(entry)
    }

    pub fn is_entry(&self, file: &ProgramFile) -> bool {
        let entry = ProgramFile::canonical(&self.root.join(&self.project.entry));
        entry == ProgramFile::canonical(file.path())
    }

    pub fn file_within(&self, path: PathBuf) -> ProgramFile {
        let tasks_directory = self.tasks_directory();
        if ProgramFile::relative_module(&path, &tasks_directory).is_some() {
            return self.task_file(ProgramFile::within(path, tasks_directory));
        }
        let scope = SearchScope::in_project(&tasks_directory);
        ProgramFile::inside_or_beside(path, self.source_root())
            .with_libraries(self.libraries.clone())
            .within_scope(scope)
    }

    pub fn test_file(&self, file: ProgramFile) -> ProgramFile {
        let scope = SearchScope::tests(&self.tasks_directory());
        self.beside_source_root(file).within_scope(scope)
    }

    pub fn task_file(&self, file: ProgramFile) -> ProgramFile {
        let scope = SearchScope::tasks(&self.tasks_directory(), &self.tests_directory());
        self.beside_source_root(file).within_scope(scope)
    }

    fn beside_source_root(&self, file: ProgramFile) -> ProgramFile {
        file.with_fallback_root(self.source_root())
            .with_libraries(self.libraries.clone())
    }

    pub fn tests_directory(&self) -> PathBuf {
        self.root.join(SearchScope::TESTS_DIRECTORY)
    }

    pub fn tasks_directory(&self) -> PathBuf {
        self.root.join(SearchScope::TASKS_DIRECTORY)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_root(&self) -> PathBuf {
        ProgramFile::directory_of(&self.root.join(&self.project.entry))
    }

    pub fn refuse_crates(&self, command: &str, terminal: &Terminal) -> Result<(), ExitStatus> {
        if !self.crates.has_crates() {
            return Ok(());
        }
        let mut scratch = terminal.scratch();
        let project = scratch.load_project(&self.project_file())?;
        let mut problems = Diagnostics::default();
        problems.push(self.crate_use(&project, command));
        Err(scratch.report(&problems))
    }

    fn crate_use(&self, project: &Project, command: &str) -> Diagnostic {
        if let Some(dependency) = project.dependencies.first() {
            let label = format!("this project uses crate `{}`", dependency.name);
            return ProjectProblems::crates_unavailable_to(command, dependency.name_span, label);
        }
        let library = project
            .libraries
            .iter()
            .find(|library| Some(&library.name) == self.crate_library.as_ref())
            .expect("a project with crates and no crate of its own lists a library that adds them");
        let label = format!("library `{}` adds crates", library.name);
        ProjectProblems::crates_unavailable_to(command, library.name_span, label)
    }

    pub fn own_crate(&self, section: CrateSection, name: &str) -> Option<&CrateDependency> {
        section
            .listed(&self.project)
            .iter()
            .find(|dependency| dependency.name == name)
    }

    pub fn project_file(&self) -> PathBuf {
        self.root.join(Language::project_file_name())
    }

    pub fn build_directory(&self) -> PathBuf {
        self.crates.build_directory()
    }

    pub fn crates(&self) -> &ProjectCrates {
        &self.crates
    }
}
