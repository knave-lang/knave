mod source_loader;

use std::rc::Rc;

use super::checking::Checking;
use super::crate_build::CrateBuild;
use super::diagnostic_output::DiagnosticOutput;
use super::emission::Emission;
use super::exit_status::ExitStatus;
use super::expansion::Expansion;
use super::interruptible_formatting::InterruptibleFormatting;
use super::program_file::ProgramFile;
use super::rustfmt_time_limit::RustfmtTimeLimit;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use super::terminal_settings::TerminalSettings;
use crate::emit::EmittedCrate;
use crate::source::{SourceFile, SourceId, SourceMap};
use crate::types::TypedProgram;
pub use source_loader::SourceLoader;

pub struct Terminal {
    sources: SourceMap,
    work: StageWork,
    settings: TerminalSettings,
}

impl Terminal {
    pub fn new(settings: TerminalSettings) -> Self {
        Self {
            sources: SourceMap::default(),
            work: StageWork::default(),
            settings,
        }
    }

    pub fn scratch(&self) -> Self {
        Self::new(TerminalSettings::reporting_to(self.diagnostic_output()))
    }

    pub fn configure(&mut self, change: impl FnOnce(&TerminalSettings) -> TerminalSettings) {
        self.settings = change(&self.settings);
    }

    pub fn diagnostic_output(&self) -> Rc<dyn DiagnosticOutput> {
        Rc::clone(self.settings.diagnostics())
    }

    pub fn work(&self) -> &StageWork {
        &self.work
    }

    pub fn sources(&mut self) -> SourceLoader<'_> {
        SourceLoader::new(&mut self.sources, self.settings.diagnostics().as_ref())
    }

    pub fn source_file(&self, source: SourceId) -> &SourceFile {
        self.sources.file(source)
    }

    pub fn expansion(&mut self) -> Expansion<'_> {
        Expansion::new(
            &mut self.sources,
            &mut self.work,
            self.settings.diagnostics().as_ref(),
        )
    }

    pub fn checking(&mut self) -> Checking<'_> {
        Checking::new(
            SourceReport::new(&self.sources, self.settings.diagnostics().as_ref()),
            &mut self.work,
        )
    }

    pub fn emission(&mut self) -> Result<Emission<'_>, ExitStatus> {
        let limit = Self::rustfmt_time_limit()?;
        Ok(self.emission_within(&limit))
    }

    pub fn source_report(&self) -> SourceReport<'_> {
        SourceReport::new(&self.sources, self.settings.diagnostics().as_ref())
    }

    pub fn crate_build(&self) -> CrateBuild<'_> {
        CrateBuild::new(self.source_report(), self.settings.interruption())
    }

    pub fn emit_file(&mut self, file: &ProgramFile) -> Result<EmittedCrate, ExitStatus> {
        let limit = Self::rustfmt_time_limit()?;
        let program = self.check_file(file)?;
        self.emission_within(&limit).emit(&program)
    }

    fn emission_within(&mut self, limit: &RustfmtTimeLimit) -> Emission<'_> {
        Emission::new(
            SourceReport::new(&self.sources, self.settings.diagnostics().as_ref()),
            &mut self.work,
            InterruptibleFormatting::new(limit.formatting(), self.settings.interruption()),
        )
    }

    fn rustfmt_time_limit() -> Result<RustfmtTimeLimit, ExitStatus> {
        RustfmtTimeLimit::from_environment().map_err(|malformed| malformed.report())
    }

    pub fn check_file(&mut self, file: &ProgramFile) -> Result<TypedProgram, ExitStatus> {
        let entry = self.sources().load(file.path())?;
        let program = self.expansion().expand(file, entry)?;
        self.checking().check(&program)
    }
}
