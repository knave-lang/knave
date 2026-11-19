use super::diagnostic_output::DiagnosticOutput;
use super::exit_status::ExitStatus;
use super::leveled_diagnostics::LeveledDiagnostics;
use super::lint_levels::LintLevels;
use crate::diagnostic::{DiagnosticCode, Diagnostics};
use crate::project::LintLevel;
use crate::source::SourceMap;

pub struct LintReport<'a> {
    sources: &'a SourceMap,
    output: &'a dyn DiagnosticOutput,
    lint_levels: &'a LintLevels,
}

impl<'a> LintReport<'a> {
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

    #[must_use]
    pub fn print(&self, diagnostics: &Diagnostics) -> ExitStatus {
        let leveled = LeveledDiagnostics::new(diagnostics, self.lint_levels);
        match self.output.print(self.sources, &leveled.shown()) {
            ExitStatus::Success if leveled.fails_lint() => ExitStatus::ProgramErrors,
            printed => printed,
        }
    }

    pub fn denied(&self, diagnostics: &Diagnostics) -> Option<Diagnostics> {
        LeveledDiagnostics::new(diagnostics, self.lint_levels).denied()
    }

    pub fn level_of(&self, code: &DiagnosticCode) -> Option<LintLevel> {
        self.lint_levels.level_of(code)
    }
}

#[cfg(test)]
mod tests {
    use super::LintReport;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::lint_levels::LintLevels;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics, Label};
    use crate::project::LintLevel;
    use crate::source::{SourceId, SourceMap, Span};

    struct LintReportFixture {
        sources: SourceMap,
        output: RecordedOutput,
        source: SourceId,
        levels: LintLevels,
    }

    impl LintReportFixture {
        const MODULE: &str = "geometry.knv";
        const TEXT: &str = "(def width 1)";

        fn leveling(levels: &[(DiagnosticCode, LintLevel)]) -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add(Self::MODULE, Self::TEXT);
            Self {
                sources,
                output: RecordedOutput::default(),
                source,
                levels: levels.iter().cloned().collect(),
            }
        }

        fn report(&self) -> LintReport<'_> {
            LintReport::new(&self.sources, &self.output, &self.levels)
        }

        fn unused_binding(&self) -> Diagnostics {
            let start = Span::new(self.source, 0..0);
            Diagnostics::from(Diagnostic::warning(
                DiagnosticCode::UNUSED_BINDING,
                "a fake unused binding",
                Label::new(start, "here"),
            ))
        }
    }

    #[test]
    fn prints_a_lint_with_no_level_as_a_failure_of_knave_lint() {
        let fixture = LintReportFixture::leveling(&[]);
        let status = fixture.report().print(&fixture.unused_binding());
        assert_eq!(status, ExitStatus::ProgramErrors);
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn prints_a_warned_lint_as_a_pass_of_knave_lint() {
        let fixture =
            LintReportFixture::leveling(&[(DiagnosticCode::UNUSED_BINDING, LintLevel::Warn)]);
        let status = fixture.report().print(&fixture.unused_binding());
        assert_eq!(status, ExitStatus::Success);
    }

    #[test]
    fn hands_back_a_denied_lint() {
        let fixture =
            LintReportFixture::leveling(&[(DiagnosticCode::UNUSED_BINDING, LintLevel::Deny)]);
        let denied = fixture.report().denied(&fixture.unused_binding());
        assert_eq!(denied.map(|denied| denied.len()), Some(1));
    }

    #[test]
    fn resolves_an_unwritten_needless_type_level_to_allow() {
        let fixture = LintReportFixture::leveling(&[]);
        let level = fixture.report().level_of(&DiagnosticCode::NEEDLESS_TYPE);
        assert_eq!(level, Some(LintLevel::Allow));
    }

    #[test]
    fn resolves_a_written_level_to_the_written_one() {
        let fixture =
            LintReportFixture::leveling(&[(DiagnosticCode::NEEDLESS_TYPE, LintLevel::Warn)]);
        let level = fixture.report().level_of(&DiagnosticCode::NEEDLESS_TYPE);
        assert_eq!(level, Some(LintLevel::Warn));
    }
}
