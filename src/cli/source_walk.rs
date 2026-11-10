use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::entry_tree::TreeFile;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::terminal::Terminal;
use crate::language::Language;

pub struct SourceWalk {
    root: PathBuf,
    skipped: SkippedDirectory,
    left_out: Vec<PathBuf>,
}

impl SourceWalk {
    pub fn of(project: &LocatedProject) -> Self {
        let root = Self::walked_root(project);
        let left_out = [project.tasks_directory(), project.tests_directory()]
            .iter()
            .filter_map(|directory| ProgramFile::canonical(directory))
            .collect();
        Self {
            skipped: SkippedDirectory::for_walk_of(&root),
            root,
            left_out,
        }
    }

    pub fn module_paths(&self, terminal: &mut Terminal) -> Result<Vec<String>, ExitStatus> {
        let files = self
            .files()
            .map_err(|error| Terminal::report_unreadable(&self.root, &error))?;
        if let Some(file) = files.iter().find(|file| file.is_unnamable()) {
            return Err(file.report_unnamable());
        }
        TreeFile::refuse_compiler_modules(&files, terminal)?;
        Ok(files.into_iter().map(TreeFile::into_module).collect())
    }

    fn walked_root(project: &LocatedProject) -> PathBuf {
        let source_root = project.source_root();
        if source_root.as_os_str().is_empty() {
            PathBuf::from(Terminal::WORKING_DIRECTORY)
        } else {
            source_root
        }
    }

    fn files(&self) -> io::Result<Vec<TreeFile>> {
        let mut files = Vec::new();
        self.collect(&self.root, &mut files)?;
        files.sort();
        Ok(files)
    }

    fn collect(&self, directory: &Path, files: &mut Vec<TreeFile>) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                self.collect_inside(&path, files)?;
            } else if let Some(module) = self.module_of(&path) {
                files.push(TreeFile::new(module, path));
            }
        }
        Ok(())
    }

    fn collect_inside(&self, directory: &Path, files: &mut Vec<TreeFile>) -> io::Result<()> {
        if self.walks_into(directory) {
            self.collect(directory, files)?;
        }
        Ok(())
    }

    fn walks_into(&self, directory: &Path) -> bool {
        let inside = ProgramFile::canonical(directory)
            .zip(ProgramFile::canonical(&self.root))
            .filter(|(directory, root)| directory.starts_with(root));
        match inside {
            Some((directory, _)) => {
                !self.skipped.is_skipped(&directory)
                    && !self.left_out.contains(&directory)
                    && !Self::holds_a_project(&directory)
            }
            None => false,
        }
    }

    fn holds_a_project(directory: &Path) -> bool {
        directory.join(Language::project_file_name()).is_file()
    }

    fn module_of(&self, file: &Path) -> Option<String> {
        if file.extension()? != Language::EXTENSION || ProgramFile::is_project_file(file) {
            return None;
        }
        ProgramFile::module_name(file.strip_prefix(&self.root).ok()?)
    }
}
