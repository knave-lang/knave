use std::path::{Path, PathBuf};

use super::entry_span::EntrySpan;
use super::module_alone::ModuleAlone;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use super::trial_stages::TrialStages;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::lint::{ProgramPosition, ProgramRun};
use crate::source::SourceId;
use crate::types::TypedProgram;

pub struct TrialRuns<'t> {
    terminal: &'t mut Terminal,
    files: &'t [ProgramFile],
}

impl<'t> TrialRuns<'t> {
    pub fn new(terminal: &'t mut Terminal, files: &'t [ProgramFile]) -> Self {
        Self { terminal, files }
    }
}

impl ProgramRun for TrialRuns<'_> {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        TrialStages::check(program)
    }

    fn emit(&self, typed: &TypedProgram) -> Option<String> {
        TrialStages::emit(typed, EntrySpan::start())
    }

    fn module_alone(&mut self, beside: ProgramPosition, module: &ModulePath) -> Option<Program> {
        ModuleAlone::new(&self.files[beside.index()])
            .within_silently(self.terminal, module.as_str(), |session| session.finish())
            .ok()
    }

    fn source_path(&self, source: SourceId) -> Option<PathBuf> {
        ProgramFile::canonical(Path::new(self.terminal.source_file(source).name()))
    }
}
