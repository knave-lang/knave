use super::entry_span::EntrySpan;
use super::exit_status::ExitStatus;
use super::interruptible_formatting::InterruptibleFormatting;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use crate::emit::{EmittedCrate, RustEmitter};
use crate::types::TypedProgram;

pub struct Emission<'a> {
    report: SourceReport<'a>,
    work: &'a mut StageWork,
    formatting: InterruptibleFormatting<'a>,
}

impl<'a> Emission<'a> {
    pub fn new(
        report: SourceReport<'a>,
        work: &'a mut StageWork,
        formatting: InterruptibleFormatting<'a>,
    ) -> Self {
        Self {
            report,
            work,
            formatting,
        }
    }

    pub fn emit(&mut self, program: &TypedProgram) -> Result<EmittedCrate, ExitStatus> {
        let emitted = RustEmitter::emit(program, EntrySpan::start())
            .map_err(|diagnostics| self.report.report(&diagnostics))?;
        self.work.emitted_expressions = Some(emitted.emission.emitted_expressions);
        let written = self.formatting.format_or_keep_unformatted(emitted)?;
        self.work.scanned_tokens = written
            .formatting
            .map(|formatting| formatting.scanned_tokens);
        Ok(written)
    }
}
