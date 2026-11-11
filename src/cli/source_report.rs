use super::diagnostic_output::DiagnosticOutput;
use super::exit_status::ExitStatus;
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

pub struct SourceReport<'a> {
    sources: &'a SourceMap,
    output: &'a dyn DiagnosticOutput,
}

impl<'a> SourceReport<'a> {
    pub fn new(sources: &'a SourceMap, output: &'a dyn DiagnosticOutput) -> Self {
        Self { sources, output }
    }

    pub fn report(&self, diagnostics: &Diagnostics) -> ExitStatus {
        self.output.report(self.sources, diagnostics)
    }

    pub fn print(&self, diagnostics: &Diagnostics) {
        if diagnostics.is_empty() {
            return;
        }
        self.output.write(self.sources, diagnostics);
    }

    pub fn note(&self, message: &str) {
        self.output.write_note(self.sources, message);
    }
}
