use super::problem_mark::ProblemMark;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::interpret::Warnings;

#[derive(Default)]
pub(super) struct ProblemReport {
    errors: Diagnostics,
    warnings: Diagnostics,
}

impl ProblemReport {
    pub(super) fn report(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }

    pub(super) fn report_all(&mut self, errors: Diagnostics) {
        self.errors.extend(errors);
    }

    pub(super) fn mark(&self) -> ReportMark {
        ReportMark {
            errors: ProblemMark::at(self.errors.len()),
            warnings: ProblemMark::at(self.warnings.len()),
        }
    }

    pub(super) fn roll_back(&mut self, mark: ReportMark) {
        drop(self.errors.split_off(mark.errors.position()));
        drop(self.warnings.split_off(mark.warnings.position()));
    }

    pub(super) fn relabel_warnings_since(
        &mut self,
        mark: ReportMark,
        relabel: impl FnMut(Diagnostic) -> Diagnostic,
    ) {
        let since = self.warnings.split_off(mark.warnings.position());
        self.warnings.extend(since.map(relabel));
    }

    pub(super) fn take_errors(&mut self) -> Diagnostics {
        std::mem::take(&mut self.errors)
    }

    pub(super) fn take_warnings(&mut self) -> Diagnostics {
        std::mem::take(&mut self.warnings)
    }
}

impl Warnings for ProblemReport {
    fn report_warning(&mut self, warning: Diagnostic) {
        self.warnings.push(warning);
    }
}

#[derive(Clone, Copy)]
pub(super) struct ReportMark {
    errors: ProblemMark,
    warnings: ProblemMark,
}

#[cfg(test)]
mod tests {
    use super::ProblemReport;
    use crate::diagnostic::{Diagnostic, Diagnostics};
    use crate::expand::problems::ExpandProblems;
    use crate::interpret::Warnings;
    use crate::source::{SourceId, Span};

    struct ReportFixture {
        report: ProblemReport,
    }

    impl ReportFixture {
        const POINTED: &str = "pointed at the call";

        fn new() -> Self {
            Self {
                report: ProblemReport::default(),
            }
        }

        fn problem() -> Diagnostic {
            ExpandProblems::empty_call(Span::new(SourceId::new(0), 0..2))
        }

        fn report_one(&mut self) {
            self.report.report(Self::problem());
        }

        fn warn_once(&mut self) {
            self.report.report_warning(Self::problem());
        }

        fn pointed_warnings(&mut self) -> Vec<bool> {
            self.report
                .take_warnings()
                .in_source_order()
                .iter()
                .map(|warning| warning.notes().iter().any(|note| note == Self::POINTED))
                .collect()
        }
    }

    #[test]
    fn reported_errors_are_taken_once() {
        let mut fixture = ReportFixture::new();
        fixture.report_one();
        assert_eq!(fixture.report.take_errors().len(), 1);
        assert!(fixture.report.take_errors().is_empty());
    }

    #[test]
    fn errors_reported_together_are_each_kept() {
        let mut fixture = ReportFixture::new();
        let mut errors = Diagnostics::default();
        errors.push(ReportFixture::problem());
        errors.push(ReportFixture::problem());
        fixture.report.report_all(errors);
        assert_eq!(fixture.report.take_errors().len(), 2);
    }

    #[test]
    fn warnings_stay_apart_from_errors() {
        let mut fixture = ReportFixture::new();
        fixture.warn_once();
        assert!(fixture.report.take_errors().is_empty());
        assert_eq!(fixture.report.take_warnings().len(), 1);
    }

    #[test]
    fn warnings_are_taken_once() {
        let mut fixture = ReportFixture::new();
        fixture.warn_once();
        fixture.report.take_warnings();
        assert!(fixture.report.take_warnings().is_empty());
    }

    #[test]
    fn rolling_back_drops_the_errors_since_the_mark() {
        let mut fixture = ReportFixture::new();
        fixture.report_one();
        let mark = fixture.report.mark();
        fixture.report_one();
        fixture.report.roll_back(mark);
        assert_eq!(fixture.report.take_errors().len(), 1);
    }

    #[test]
    fn rolling_back_drops_the_warnings_since_the_mark() {
        let mut fixture = ReportFixture::new();
        fixture.warn_once();
        let mark = fixture.report.mark();
        fixture.warn_once();
        fixture.report.roll_back(mark);
        assert_eq!(fixture.report.take_warnings().len(), 1);
    }

    #[test]
    fn relabeling_changes_only_the_warnings_since_the_mark() {
        let mut fixture = ReportFixture::new();
        fixture.warn_once();
        let mark = fixture.report.mark();
        fixture.warn_once();
        fixture
            .report
            .relabel_warnings_since(mark, |warning| warning.with_note(ReportFixture::POINTED));
        assert_eq!(fixture.pointed_warnings(), [false, true]);
    }
}
