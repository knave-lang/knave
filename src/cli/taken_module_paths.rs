use super::exit_status::ExitStatus;
use super::terminal::Terminal;
use super::tree_file::TreeFile;
use crate::core::ModulePath;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::expand::TakenModuleProblems;
use crate::modules::{LibraryTable, ModuleSources};
use crate::source::Span;

pub struct TakenModulePaths<'l> {
    libraries: Option<&'l LibraryTable>,
}

enum ModuleOwner {
    Compiler,
    Library,
}

impl<'l> TakenModulePaths<'l> {
    pub fn by_the_compiler() -> Self {
        Self { libraries: None }
    }

    pub fn by_the_compiler_and(libraries: &'l LibraryTable) -> Self {
        Self {
            libraries: Some(libraries),
        }
    }

    pub fn refuse(&self, files: &[TreeFile], terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let mut problems = Diagnostics::default();
        for file in files {
            let path = file.module_path();
            if let Some(owner) = self.owner_of(&path) {
                let start = Span::new(terminal.sources().load(file.path())?, 0..0);
                problems.push(owner.problem(&path, start));
            }
        }
        if problems.has_errors() {
            return Err(terminal.source_report().report(&problems));
        }
        Ok(())
    }

    fn owner_of(&self, path: &ModulePath) -> Option<ModuleOwner> {
        if !ModuleSources::is_written(path) {
            return Some(ModuleOwner::Compiler);
        }
        self.libraries
            .filter(|libraries| libraries.names_library(None, path))
            .map(|_| ModuleOwner::Library)
    }
}

impl ModuleOwner {
    fn problem(&self, path: &ModulePath, start: Span) -> Diagnostic {
        match self {
            Self::Compiler => TakenModuleProblems::file_takes_compiler_module(path, start),
            Self::Library => TakenModuleProblems::file_takes_library_module(path, start),
        }
    }
}
