use std::path::{Path, PathBuf};

use super::doc_folder::DocFolder;
use super::doc_pages::DocumentedModules;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::terminal::Terminal;

pub struct DocWriteCommand {
    output: Option<PathBuf>,
}

impl DocWriteCommand {
    const DEFAULT_FOLDER: &'static str = "doc";
    const ENTRY: &'static str = "<documentation>";

    pub fn new(output: Option<PathBuf>) -> Self {
        Self { output }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self.write(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn write(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let project =
            LocatedProject::search_from(Path::new(Terminal::WORKING_DIRECTORY), terminal)?
                .ok_or_else(LocatedProject::report_no_path)?;
        let folder = DocFolder::new(self.folder(&project)).writable()?;
        let pages = Self::modules(project)?.pages(terminal)?;
        Ok(folder.replace_pages(&pages))
    }

    fn folder(&self, project: &LocatedProject) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| project.root().join(Self::DEFAULT_FOLDER))
    }

    fn modules(project: LocatedProject) -> Result<DocumentedModules, ExitStatus> {
        let file = project.file_within(project.source_root().join(Self::ENTRY));
        let tree = EntryTree::of_tests(project.source_root(), Some(project));
        Ok(DocumentedModules::new(file, tree.module_paths()?))
    }
}
