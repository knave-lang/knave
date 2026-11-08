use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::library_walk::LibraryWalk;
use super::program_file::ProgramFile;
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
        let Some(start) = ProgramFile::canonical(directory) else {
            return Ok(None);
        };
        let file_name = Language::project_file_name();
        let found = start
            .ancestors()
            .find(|ancestor| ancestor.join(&file_name).is_file());
        match found {
            Some(root) => Ok(Some(Self::shown(root)?)),
            None => Ok(None),
        }
    }

    fn shown(root: &Path) -> Result<PathBuf, ExitStatus> {
        let working_directory = Terminal::canonical(Path::new(Terminal::WORKING_DIRECTORY))?;
        Ok(root
            .strip_prefix(&working_directory)
            .unwrap_or(root)
            .to_path_buf())
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

    pub fn source_root(&self) -> PathBuf {
        ProgramFile::directory_of(&self.root.join(&self.project.entry))
    }

    pub fn build_directory(&self) -> PathBuf {
        self.root
            .join(Terminal::build_root())
            .join(&self.project.name)
    }
}
