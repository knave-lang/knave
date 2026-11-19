use super::diagnostic_output::DiagnosticOutput;
use super::exit_status::ExitStatus;
use super::leveled_diagnostics::LeveledDiagnostics;
use super::lint_levels::LintLevels;
use super::lint_report::LintReport;
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

pub struct SourceReport<'a> {
    sources: &'a SourceMap,
    output: &'a dyn DiagnosticOutput,
    lint_levels: &'a LintLevels,
}

impl<'a> SourceReport<'a> {
    pub fn new(
        sources: &'a SourceMap,
        output: &'a dyn DiagnosticOutput,
        lint_levels: &'a LintLevels,
    ) -> Self {
        Self {
            sources,
            output,
            lint_levels,
        }
    }

    pub fn sources(&self) -> &'a SourceMap {
        self.sources
    }

    pub fn lints(&self) -> LintReport<'a> {
        LintReport::new(self.sources, self.output, self.lint_levels)
    }

    pub fn report(&self, diagnostics: &Diagnostics) -> ExitStatus {
        let leveled = LeveledDiagnostics::new(diagnostics, self.lint_levels);
        self.output.report(self.sources, &leveled.shown())
    }

    #[must_use]
    pub fn print(&self, diagnostics: &Diagnostics) -> ExitStatus {
        let leveled = LeveledDiagnostics::new(diagnostics, self.lint_levels);
        self.output.print(self.sources, &leveled.shown())
    }

    pub fn note(&self, message: &str) {
        self.output.write_note(self.sources, message);
    }
}

#[cfg(test)]
mod tests {
    use super::SourceReport;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::lint_levels::LintLevels;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics, Label};
    use crate::project::LintLevel;
    use crate::reader::Reader;
    use crate::source::{SourceId, SourceMap, Span};

    struct ReportFixture {
        sources: SourceMap,
        output: RecordedOutput,
        source: SourceId,
        levels: LintLevels,
    }

    impl ReportFixture {
        const MODULE: &str = "geometry.knv";
        const UNCLOSED: &str = "(area 2 3";

        fn new() -> Self {
            Self::leveling_unused_bindings(&[])
        }

        fn leveling_unused_bindings(level: &[LintLevel]) -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add(Self::MODULE, Self::UNCLOSED);
            let levels = level
                .iter()
                .map(|level| (DiagnosticCode::UNUSED_BINDING, *level))
                .collect();
            Self {
                sources,
                output: RecordedOutput::default(),
                source,
                levels,
            }
        }

        fn report(&self) -> SourceReport<'_> {
            SourceReport::new(&self.sources, &self.output, &self.levels)
        }

        fn unused_binding(&self) -> Diagnostics {
            let start = Span::new(self.source, 0..0);
            Diagnostics::from(Diagnostic::warning(
                DiagnosticCode::UNUSED_BINDING,
                "a fake unused binding",
                Label::new(start, "here"),
            ))
        }

        fn errors(&self) -> Diagnostics {
            Reader::new(self.sources.file(self.source))
                .read()
                .expect_err("an unclosed list does not read")
        }

        fn errors_and_an_unused_binding(&self) -> Diagnostics {
            let mut diagnostics = self.errors();
            diagnostics.extend(self.unused_binding());
            diagnostics
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
    fn reports_no_allowed_lint_beside_an_error() {
        let fixture = ReportFixture::leveling_unused_bindings(&[LintLevel::Allow]);
        let status = fixture
            .report()
            .report(&fixture.errors_and_an_unused_binding());
        assert_eq!(status, ExitStatus::ProgramErrors);
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn prints_a_lint_with_no_level_as_a_pass_of_every_other_command() {
        let fixture = ReportFixture::new();
        let status = fixture.report().print(&fixture.unused_binding());
        assert_eq!(status, ExitStatus::Success);
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
        let status = fixture.report().print(&fixture.notes());
        assert_eq!(status, ExitStatus::Success);
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn prints_nothing_for_no_diagnostics() {
        let fixture = ReportFixture::new();
        let status = fixture.report().print(&Diagnostics::default());
        assert_eq!(status, ExitStatus::Success);
        assert!(fixture.output.writes().is_empty());
    }

    #[test]
    fn writes_a_note() {
        let fixture = ReportFixture::new();
        fixture.report().note("waiting for a fake build");
        assert_eq!(fixture.output.writes(), ["note: waiting for a fake build"]);
    }
}
