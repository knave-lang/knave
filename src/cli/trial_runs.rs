use super::entry_span::EntrySpan;
use super::module_alone::ModuleAlone;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use super::trial_stages::TrialStages;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::lint::ProgramRun;
use crate::types::TypedProgram;

pub struct TrialRuns<'t> {
    terminal: &'t mut Terminal,
    file: &'t ProgramFile,
}

impl<'t> TrialRuns<'t> {
    pub fn new(terminal: &'t mut Terminal, file: &'t ProgramFile) -> Self {
        Self { terminal, file }
    }
}

impl ProgramRun for TrialRuns<'_> {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        TrialStages::check(program)
    }

    fn emit(&self, typed: &TypedProgram) -> Option<String> {
        TrialStages::emit(typed, EntrySpan::start())
    }

    fn module_alone(&mut self, module: &ModulePath) -> Option<Program> {
        ModuleAlone::new(self.file)
            .within_silently(self.terminal, module.as_str(), |session| session.finish())
            .ok()
    }
}
