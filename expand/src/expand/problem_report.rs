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
