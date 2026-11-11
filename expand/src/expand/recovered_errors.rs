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

#[cfg(test)]
mod tests {
    use super::RecoveredErrors;
    use crate::diagnostic::Diagnostic;
    use crate::expand::problems::ExpandProblems;
    use crate::scope::Phase;
    use crate::source::{SourceId, Span};

    struct RecoveryFixture {
        recovered: RecoveredErrors,
    }

    impl RecoveryFixture {
        fn new() -> Self {
            Self {
                recovered: RecoveredErrors::default(),
            }
        }

        fn error_at(start: usize) -> Diagnostic {
            ExpandProblems::empty_call(Span::new(SourceId::new(0), start..start + 2))
        }

        fn keep_at(&mut self, start: usize) {
            let kept = self.recovered.keep(Self::error_at(start), Phase::Runtime);
            assert!(kept.is_ok());
        }

        fn starts(&mut self) -> Vec<usize> {
            self.recovered
                .take_all()
                .iter()
                .map(|error| error.primary().span().range().start)
                .collect()
        }
    }

    #[test]
    fn an_error_at_runtime_is_kept() {
        let mut fixture = RecoveryFixture::new();
        fixture.keep_at(4);
        assert_eq!(fixture.starts(), [4]);
    }

    #[test]
    fn taking_every_error_leaves_none() {
        let mut fixture = RecoveryFixture::new();
        fixture.keep_at(4);
        fixture.starts();
        assert!(fixture.starts().is_empty());
    }

    #[test]
    fn an_error_at_compile_time_is_refused_and_not_kept() {
        let mut fixture = RecoveryFixture::new();
        let refused = fixture
            .recovered
            .keep(RecoveryFixture::error_at(4), Phase::CompileTime);
        assert!(refused.is_err());
        assert!(fixture.starts().is_empty());
    }

    #[test]
    fn settling_a_success_keeps_one_of_each_repeated_error() {
        let mut fixture = RecoveryFixture::new();
        let mark = fixture.recovered.mark();
        fixture.keep_at(4);
        fixture.keep_at(4);
        fixture.keep_at(8);
        let settled = fixture.recovered.settle_since(mark, Ok(()));
        assert!(settled.is_ok());
        assert_eq!(fixture.starts(), [4, 8]);
    }

    #[test]
    fn settling_a_failure_returns_it_and_keeps_the_other_errors_beside_it() {
        let mut fixture = RecoveryFixture::new();
        let mark = fixture.recovered.mark();
        fixture.keep_at(4);
        fixture.keep_at(8);
        let settled: Result<(), Diagnostic> = fixture
            .recovered
            .settle_since(mark, Err(RecoveryFixture::error_at(4)));
        let failure = settled.expect_err("the failure stays a failure");
        assert_eq!(failure.primary().span().range().start, 4);
        assert_eq!(fixture.starts(), [8]);
    }

    #[test]
    fn settling_leaves_errors_before_the_mark_alone() {
        let mut fixture = RecoveryFixture::new();
        fixture.keep_at(4);
        let mark = fixture.recovered.mark();
        fixture.keep_at(4);
        let _ = fixture.recovered.settle_since(mark, Ok(()));
        assert_eq!(fixture.starts(), [4, 4]);
    }

    #[test]
    fn the_first_error_since_a_mark_is_taken_and_the_rest_stay() {
        let mut fixture = RecoveryFixture::new();
        fixture.keep_at(2);
        let mark = fixture.recovered.mark();
        fixture.keep_at(4);
        fixture.keep_at(8);
        let first = fixture.recovered.take_first_since(mark);
        assert_eq!(
            first.map(|error| error.primary().span().range().start),
            Some(4)
        );
        assert_eq!(fixture.starts(), [2, 8]);
    }

    #[test]
    fn no_error_is_taken_when_none_came_since_the_mark() {
        let mut fixture = RecoveryFixture::new();
        fixture.keep_at(2);
        let mark = fixture.recovered.mark();
        assert!(fixture.recovered.take_first_since(mark).is_none());
    }

    #[test]
    fn rolling_back_drops_the_errors_since_the_mark() {
        let mut fixture = RecoveryFixture::new();
        fixture.keep_at(2);
        let mark = fixture.recovered.mark();
        fixture.keep_at(4);
        fixture.recovered.roll_back(mark);
        assert_eq!(fixture.starts(), [2]);
    }
}
