use std::path::{Path, PathBuf};

use super::doc_folder::DocFolder;
use super::doc_pages::DocumentedModules;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::source_walk::SourceWalk;
use super::terminal::Terminal;
use crate::modules::PublicPrelude;

pub struct DocWriteCommand {
    output: Option<PathBuf>,
    scope: DocScope,
}

pub enum DocScope {
    Project,
    Prelude,
}

impl DocWriteCommand {
    pub const DEFAULT_FOLDER: &'static str = "doc";

    pub fn new(output: Option<PathBuf>, scope: DocScope) -> Self {
        Self { output, scope }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self.write(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn write(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let (folder, modules) = match self.scope {
            DocScope::Project => self.project_target(terminal)?,
            DocScope::Prelude => self.prelude_target(),
        };
        let folder = DocFolder::new(folder).writable()?;
        let pages = modules.pages(terminal)?;
        Ok(folder.replace_pages(&pages))
    }

    fn project_target(
        &self,
        terminal: &mut Terminal,
    ) -> Result<(PathBuf, DocumentedModules), ExitStatus> {
        let project =
            LocatedProject::search_from(Path::new(Terminal::WORKING_DIRECTORY), terminal)?
                .ok_or_else(LocatedProject::report_no_path)?;
        let folder = self.folder_in(project.root());
        let file = project.file_within(project.source_root().join(DocumentedModules::CALL_SOURCE));
        let modules = SourceWalk::of(&project).module_paths(terminal)?;
        let modules = DocumentedModules::new(file, modules).with_entry(project.entry());
        Ok((folder, modules))
    }

    fn prelude_target(&self) -> (PathBuf, DocumentedModules) {
        let root = LocatedProject::canonical_root(Path::new(Terminal::WORKING_DIRECTORY))
            .unwrap_or_default();
        let file = ProgramFile::beside_itself(PathBuf::from(DocumentedModules::CALL_SOURCE));
        let modules = DocumentedModules::new(file, PublicPrelude::paths());
        (self.folder_in(&root), modules)
    }

    fn folder_in(&self, root: &Path) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| root.join(Self::DEFAULT_FOLDER))
    }
}
