use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::library_walk::LibraryWalk;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::build::{CargoManifest, CargoTarget, CargoTargetKind};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::modules::{LibraryTable, SearchScope};
use crate::project::{CrateDependency, Project, ProjectProblems};

pub struct LocatedProject {
    root: PathBuf,
    project: Project,
    libraries: LibraryTable,
    crates: Vec<CrateDependency>,
    dev_crates: Vec<CrateDependency>,
    crate_library: Option<String>,
}

impl LocatedProject {
    const TEST_CRATE_DIRECTORY: &str = "test_runner";
    const TEST_TARGET_PREFIX: &str = "tests";
    const LOCK_FILE: &str = "Cargo.lock";
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    const SHORT_HASH_MASK: u64 = 0xffff_ffff;

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
        Ok(Some(Self {
            root,
            project,
            libraries: walked.table,
            crates: walked.crates,
            crate_library: walked.crate_library,
            dev_crates,
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

    pub fn uses_crates(&self) -> bool {
        !self.crates.is_empty()
    }

    pub fn refuse_crates(&self, command: &str, terminal: &Terminal) -> Result<(), ExitStatus> {
        if !self.uses_crates() {
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

    pub fn tests_use_crates(&self) -> bool {
        self.uses_crates() || !self.dev_crates.is_empty()
    }

    pub fn cargo_target<'t>(&'t self, output: &'t Path, bin_stem: &str) -> CargoTarget<'t> {
        let name = format!("{bin_stem}-{}", self.root_hash());
        self.target_of(CargoTargetKind::Bin, name, output)
    }

    pub fn cargo_test_target<'t>(&'t self, output: &'t Path) -> CargoTarget<'t> {
        let name = format!(
            "{}-{}-{}",
            self.project.name,
            Self::TEST_TARGET_PREFIX,
            self.root_hash()
        );
        self.target_of(CargoTargetKind::Test, name, output)
    }

    fn target_of<'t>(
        &'t self,
        target: CargoTargetKind,
        target_name: String,
        output: &'t Path,
    ) -> CargoTarget<'t> {
        CargoTarget {
            manifest: CargoManifest {
                target,
                target_name,
                dependencies: &self.crates,
                dev_dependencies: &self.dev_crates,
            },
            lock_file: self.root.join(Self::LOCK_FILE),
            output,
        }
    }

    fn root_hash(&self) -> String {
        let root = ProgramFile::canonical(&self.root).unwrap_or(self.root.clone());
        let hash = root
            .as_os_str()
            .as_encoded_bytes()
            .iter()
            .fold(Self::FNV_OFFSET, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(Self::FNV_PRIME)
            });
        format!("{:08x}", hash & Self::SHORT_HASH_MASK)
    }

    pub fn own_crate(&self, name: &str) -> Option<&CrateDependency> {
        self.project
            .dependencies
            .iter()
            .find(|dependency| dependency.name == name)
    }

    pub fn project_file(&self) -> PathBuf {
        self.root.join(Language::project_file_name())
    }

    pub fn build_directory(&self) -> PathBuf {
        self.root
            .join(Terminal::build_root())
            .join(&self.project.name)
    }

    pub fn test_crate_directory(&self) -> PathBuf {
        self.build_directory().join(Self::TEST_CRATE_DIRECTORY)
    }
}
