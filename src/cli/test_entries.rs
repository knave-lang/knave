use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::terminal::{ModuleOfSources, Terminal};
use crate::language::Language;

pub struct TestEntries {
    path: PathBuf,
}

struct TestTree {
    path: PathBuf,
    project: Option<LocatedProject>,
}

impl TestEntries {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        TestTree::at(self.path.clone(), terminal)?.load(terminal)
    }
}

impl TestTree {
    const DIRECTORY_ENTRY: &'static str = "<tests>";

    fn at(path: PathBuf, terminal: &Terminal) -> Result<Self, ExitStatus> {
        let project = LocatedProject::search_from(&path, terminal)?;
        Ok(Self { path, project })
    }

    fn load(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
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

    fn load_directory(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let mut modules = Vec::new();
        self.collect_modules(&self.path, &mut modules)
            .map_err(|error| Terminal::report_unreadable(&self.path, &error))?;
        modules.sort();
        let uses: Vec<String> = modules
            .iter()
            .map(|module| format!("(use {module})"))
            .collect();
        let source = terminal.add_named_source(Self::DIRECTORY_ENTRY, &uses.join("\n"));
        Ok(ModuleOfSources {
            file: self.directory_file(),
            sources: vec![source],
        })
    }

    fn directory_file(&self) -> ProgramFile {
        let file = ProgramFile::within(self.path.join(Self::DIRECTORY_ENTRY), self.path.clone());
        match &self.project {
            Some(project) => project.test_file(file),
            None => file,
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
        if SkippedDirectory::is_skipped(directory) {
            return Ok(());
        }
        self.collect_modules(directory, modules)
    }

    fn module_path(&self, file: &Path) -> Option<String> {
        if file.extension()? != Language::EXTENSION || ProgramFile::is_project_file(file) {
            return None;
        }
        ProgramFile::module_name(file.strip_prefix(&self.path).ok()?)
    }
}
