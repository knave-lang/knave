use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::diagnostic_output::{DiagnosticOutput, StandardErrorMessages};
use super::doc_folder::{DocFolder, WrittenPage};
use super::doc_pages::{DocumentedModules, EntryProgram};
use super::error_pages::ErrorPages;
use super::exit_status::ExitStatus;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::source_walk::SourceWalk;
use super::terminal::Terminal;
use super::unreadable_file::UnreadableFile;
use super::user_label_output::UserLabelOutput;
use super::working_directory::WorkingDirectory;
use crate::modules::{ModuleSources, PublicPrelude};

pub struct DocWriteCommand {
    output: Option<PathBuf>,
    scope: DocScope,
}

pub enum DocScope {
    Project,
    Prelude,
    Errors,
    PreludeAndErrors,
}

enum PageSet {
    Modules(Box<DocumentedModules>),
    Errors,
}

impl DocWriteCommand {
    pub const DEFAULT_FOLDER: &'static str = "doc";

    pub fn new(output: Option<PathBuf>, scope: DocScope) -> Self {
        Self { output, scope }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let output = Self::user_output(terminal.diagnostic_output());
        terminal.configure(|settings| settings.rerouted_to(output));
        match self.write(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn user_output(inner: Rc<dyn DiagnosticOutput>) -> Rc<dyn DiagnosticOutput> {
        let generated = vec![DocumentedModules::CALL_SOURCE];
        Rc::new(UserLabelOutput::new(inner, generated))
    }

    fn write(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let (folder, sets) = self.target(terminal)?;
        let folder = DocFolder::new(folder).writable()?;
        let mut pages = Vec::new();
        for set in sets {
            pages.extend(set.pages(terminal)?);
        }
        Ok(folder.replace_pages(&pages))
    }

    fn target(&self, terminal: &mut Terminal) -> Result<(PathBuf, Vec<PageSet>), ExitStatus> {
        Ok(match self.scope {
            DocScope::Project => self.project_target(terminal)?,
            DocScope::Prelude => (self.reference_folder(), vec![Self::prelude_modules()]),
            DocScope::Errors => (self.reference_folder(), vec![PageSet::Errors]),
            DocScope::PreludeAndErrors => (
                self.reference_folder(),
                vec![Self::prelude_modules(), PageSet::Errors],
            ),
        })
    }

    fn project_target(
        &self,
        terminal: &mut Terminal,
    ) -> Result<(PathBuf, Vec<PageSet>), ExitStatus> {
        let project = LocatedProject::search_from(
            WorkingDirectory::path(),
            terminal,
            FetchingStore::default(),
        )?
        .ok_or_else(LocatedProject::report_no_path)?;
        let folder = self.folder_in(project.root());
        let file = project
            .placement()
            .file_within(project.source_root().join(DocumentedModules::CALL_SOURCE));
        let modules = SourceWalk::of(&project).module_paths(terminal)?;
        let entry = project.entry();
        let text = Self::entry_text(entry.path())?;
        let entry = EntryProgram::new(entry, text);
        let modules = DocumentedModules::new(file, entry, modules);
        Ok((folder, vec![PageSet::Modules(Box::new(modules))]))
    }

    fn entry_text(path: &Path) -> Result<String, ExitStatus> {
        match ModuleSources::read_file(path) {
            Ok(text) => Ok(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            Err(error) => Err(UnreadableFile::new(path, &error).report_to(&StandardErrorMessages)),
        }
    }

    fn reference_folder(&self) -> PathBuf {
        let root = LocatedProject::canonical_root(WorkingDirectory::path()).unwrap_or_default();
        self.folder_in(&root)
    }

    fn prelude_modules() -> PageSet {
        let file = ProgramFile::beside_itself(PathBuf::from(DocumentedModules::CALL_SOURCE));
        let empty_entry = EntryProgram::new(file.clone(), String::new());
        let modules = DocumentedModules::new(file, empty_entry, PublicPrelude::paths());
        PageSet::Modules(Box::new(modules))
    }

    fn folder_in(&self, root: &Path) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| root.join(Self::DEFAULT_FOLDER))
    }
}

impl PageSet {
    fn pages(self, terminal: &mut Terminal) -> Result<Vec<WrittenPage>, ExitStatus> {
        match self {
            Self::Modules(modules) => modules.pages(terminal),
            Self::Errors => Ok(ErrorPages::pages()),
        }
    }
}
