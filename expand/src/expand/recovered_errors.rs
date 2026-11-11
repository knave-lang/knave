use super::problem_mark::ProblemMark;
use crate::diagnostic::{Diagnostic, DistinctDiagnostics};
use crate::scope::Phase;

#[derive(Default)]
pub(super) struct RecoveredErrors {
    errors: Vec<Diagnostic>,
}

impl RecoveredErrors {
    pub(super) fn keep(&mut self, error: Diagnostic, phase: Phase) -> Result<(), Diagnostic> {
        if phase == Phase::CompileTime {
            return Err(error);
        }
        self.add(error);
        Ok(())
    }

    pub(super) fn add(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }

    pub(super) fn mark(&self) -> ProblemMark {
        ProblemMark::at(self.errors.len())
    }

    pub(super) fn settle_since<T>(
        &mut self,
        mark: ProblemMark,
        outcome: Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let recovered = self.errors.split_off(mark.position());
        match outcome {
            Ok(value) => {
                self.keep_distinct(recovered);
                Ok(value)
            }
            Err(error) => Err(self.keep_distinct_beside(error, recovered)),
        }
    }

    pub(super) fn take_first_since(&mut self, mark: ProblemMark) -> Option<Diagnostic> {
        let first = mark.position();
        (self.errors.len() > first).then(|| self.errors.remove(first))
    }

    pub(super) fn roll_back(&mut self, mark: ProblemMark) {
        self.errors.truncate(mark.position());
    }

    pub(super) fn take_all(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.errors)
    }

    fn keep_distinct(&mut self, recovered: Vec<Diagnostic>) {
        let distinct = DistinctDiagnostics::of(recovered);
        self.errors.extend(distinct.into_diagnostics());
    }

    fn keep_distinct_beside(
        &mut self,
        error: Diagnostic,
        recovered: Vec<Diagnostic>,
    ) -> Diagnostic {
        let with_error = std::iter::once(error).chain(recovered);
        let mut distinct = DistinctDiagnostics::of(with_error).into_diagnostics();
        let most_helpful_error = distinct.remove(0);
        self.errors.extend(distinct);
        most_helpful_error
    }
}
