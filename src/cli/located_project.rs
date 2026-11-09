use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::library_walk::LibraryWalk;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::language::Language;
use crate::modules::LibraryTable;
use crate::project::Project;

pub struct LocatedProject {
    root: PathBuf,
    project: Project,
    libraries: LibraryTable,
}

impl LocatedProject {
    pub const TESTS_DIRECTORY: &str = "tests";

    pub fn search_from(directory: &Path, terminal: &Terminal) -> Result<Option<Self>, ExitStatus> {
        let Some(root) = Self::nearest_root(directory)? else {
            return Ok(None);
        };
        let mut scratch = terminal.scratch();
        let project = scratch.load_project(&root.join(Language::project_file_name()))?;
        let libraries = LibraryWalk::table(&mut scratch, &root, &project)?;
        Ok(Some(Self {
            root,
            project,
            libraries,
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
        ProgramFile::inside_or_beside(path, self.source_root())
            .with_libraries(self.libraries.clone())
    }

    pub fn test_file(&self, file: ProgramFile) -> ProgramFile {
        file.with_fallback_root(self.source_root())
            .with_libraries(self.libraries.clone())
    }

    pub fn tests_directory(&self) -> PathBuf {
        self.root.join(Self::TESTS_DIRECTORY)
    }

    pub fn source_root(&self) -> PathBuf {
        ProgramFile::directory_of(&self.root.join(&self.project.entry))
    }

    pub fn build_directory(&self) -> PathBuf {
        self.root
            .join(Terminal::build_root())
            .join(&self.project.name)
    }
}
