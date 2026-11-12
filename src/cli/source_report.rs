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

    pub fn sources(&self) -> &'a SourceMap {
        self.sources
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

#[cfg(test)]
mod tests {
    use super::SourceReport;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::diagnostic::{Diagnostic, Diagnostics, Label};
    use crate::reader::Reader;
    use crate::source::{SourceId, SourceMap, Span};

    struct ReportFixture {
        sources: SourceMap,
        output: RecordedOutput,
        source: SourceId,
    }

    impl ReportFixture {
        const MODULE: &str = "geometry.knv";
        const UNCLOSED: &str = "(area 2 3";

        fn new() -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add(Self::MODULE, Self::UNCLOSED);
            Self {
                sources,
                output: RecordedOutput::default(),
                source,
            }
        }

        fn report(&self) -> SourceReport<'_> {
            SourceReport::new(&self.sources, &self.output)
        }

        fn errors(&self) -> Diagnostics {
            Reader::new(self.sources.file(self.source))
                .read()
                .expect_err("an unclosed list does not read")
        }

        fn notes(&self) -> Diagnostics {
            let mut notes = Diagnostics::default();
            let start = Span::new(self.source, 0..0);
            notes.push(Diagnostic::note("a fake note", Label::new(start, "here")));
            notes
        }
    }

    #[test]
    fn reports_errors_as_program_errors() {
        let fixture = ReportFixture::new();
        let status = fixture.report().report(&fixture.errors());
        assert_eq!(status, ExitStatus::ProgramErrors);
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn reports_diagnostics_without_errors_as_success_and_writes_nothing() {
        let fixture = ReportFixture::new();
        let status = fixture.report().report(&fixture.notes());
        assert_eq!(status, ExitStatus::Success);
        assert!(fixture.output.writes().is_empty());
    }

    #[test]
    fn prints_diagnostics_that_are_not_errors() {
        let fixture = ReportFixture::new();
        fixture.report().print(&fixture.notes());
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn prints_nothing_for_no_diagnostics() {
        let fixture = ReportFixture::new();
        fixture.report().print(&Diagnostics::default());
        assert!(fixture.output.writes().is_empty());
    }

    #[test]
    fn writes_a_note() {
        let fixture = ReportFixture::new();
        fixture.report().note("waiting for a fake build");
        assert_eq!(fixture.output.writes(), ["note: waiting for a fake build"]);
    }
}
