use std::path::{Path, PathBuf};

use super::doc_folder::DocFolder;
use super::doc_pages::DocumentedModules;
use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::terminal::Terminal;

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
    const ENTRY: &'static str = "<documentation>";

    pub fn new(output: Option<PathBuf>, scope: DocScope) -> Self {
        Self { output, scope }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self.write(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn write(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let project =
            LocatedProject::search_from(Path::new(Terminal::WORKING_DIRECTORY), terminal)?;
        let folder = self.folder(project.as_ref());
        let modules = self.modules(project)?;
        let folder = DocFolder::new(folder).writable()?;
        let pages = modules.pages(terminal)?;
        Ok(folder.replace_pages(&pages))
    }

    fn folder(&self, project: Option<&LocatedProject>) -> PathBuf {
        let root = project.map_or_else(PathBuf::new, |project| project.root().to_path_buf());
        self.output
            .clone()
            .unwrap_or_else(|| root.join(Self::DEFAULT_FOLDER))
    }

    fn modules(&self, project: Option<LocatedProject>) -> Result<DocumentedModules, ExitStatus> {
        match self.scope {
            DocScope::Project => {
                Self::project_modules(project.ok_or_else(LocatedProject::report_no_path)?)
            }
            DocScope::Prelude => Ok(DocumentedModules::new(
                ProgramFile::beside_itself(PathBuf::from(Self::ENTRY)),
                DocumentedModules::public_prelude(),
            )),
        }
    }

    fn project_modules(project: LocatedProject) -> Result<DocumentedModules, ExitStatus> {
        let file = project.file_within(project.source_root().join(Self::ENTRY));
        let tree = EntryTree::of_tests(Self::walked_root(&project), Some(project));
        Ok(DocumentedModules::new(file, tree.module_paths()?))
    }

    fn walked_root(project: &LocatedProject) -> PathBuf {
        let source_root = project.source_root();
        if source_root.as_os_str().is_empty() {
            PathBuf::from(Terminal::WORKING_DIRECTORY)
        } else {
            source_root
        }
    }
}
