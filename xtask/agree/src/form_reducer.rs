use std::time::Duration;

use crate::backend::Backend;
use crate::backend_verdicts::BackendVerdicts;
use crate::finding_signature::FindingSignature;
use crate::oracle::Oracle;
use crate::reducer::Reducer;
use crate::reduction_candidates::ReductionCandidates;
use crate::subject_program::SubjectProgram;

pub struct FormReducer {
    limit: Duration,
    attempts: usize,
}

struct Reduction<'a> {
    oracle: &'a dyn Oracle,
    backends: &'a [&'a dyn Backend],
    limit: Duration,
    attempts_left: usize,
}

impl FormReducer {
    pub fn new(limit: Duration, attempts: usize) -> Self {
        Self { limit, attempts }
    }
}

impl Reducer for FormReducer {
    fn shrink(
        &self,
        program: SubjectProgram,
        oracle: &dyn Oracle,
        backends: &[&dyn Backend],
    ) -> SubjectProgram {
        let mut reduction = Reduction {
            oracle,
            backends,
            limit: self.limit,
            attempts_left: self.attempts,
        };
        let Some(target) = reduction.signature(&program) else {
            return program;
        };
        let mut smallest = program;
        let mut start = 0;
        loop {
            match reduction.smaller(&smallest, &target, start) {
                Some((smaller, position)) => {
                    smallest = smaller;
                    start = position;
                }
                None if start > 0 => start = 0,
                None => break,
            }
        }
        smallest
    }
}

impl Reduction<'_> {
    fn smaller(
        &mut self,
        program: &SubjectProgram,
        target: &FindingSignature,
        start: usize,
    ) -> Option<(SubjectProgram, usize)> {
        let candidates = ReductionCandidates::of(program.entry_text());
        for (position, text) in candidates.texts().enumerate().skip(start) {
            self.attempts_left = self.attempts_left.checked_sub(1)?;
            let candidate = program.with_entry_text(text);
            if self.signature(&candidate).as_ref() == Some(target) {
                return Some((candidate, position));
            }
        }
        None
    }

    fn signature(&self, program: &SubjectProgram) -> Option<FindingSignature> {
        let verdicts = BackendVerdicts::collect(program.clone(), self.backends, self.limit);
        self.oracle
            .judge(&verdicts)
            .map(|finding| finding.signature())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use knave_expand::diagnostic::DiagnosticCode;

    use super::FormReducer;
    use crate::backend::Backend;
    use crate::backend_verdicts::BackendVerdicts;
    use crate::citation::Citation;
    use crate::finding::Finding;
    use crate::oracle::Oracle;
    use crate::reducer::Reducer;
    use crate::subject_program::SubjectProgram;
    use crate::verdict::{RejectingTool, Rejection, Verdict};

    struct MarkerBackend;

    struct MarkerOracle;

    struct ReducerFixture;

    impl ReducerFixture {
        const MARKER: &str = "(bad)";
        const LIMIT: Duration = Duration::from_secs(1);

        fn program(text: &str) -> SubjectProgram {
            SubjectProgram::of_entry(
                "probe".to_string(),
                PathBuf::from("probe.knv"),
                text.to_string(),
            )
        }

        fn rejection() -> Rejection {
            Rejection {
                tool: RejectingTool::Rustc,
                code: Some(DiagnosticCode::from_text("E0502".to_string())),
                file: PathBuf::from("main.rs"),
                span: 0..0,
                function: Some("main".to_string()),
                reached_from_main: true,
            }
        }

        fn shrunk(text: &str, attempts: usize) -> String {
            let backends: [&dyn Backend; 1] = [&MarkerBackend];
            FormReducer::new(Self::LIMIT, attempts)
                .shrink(Self::program(text), &MarkerOracle, &backends)
                .entry_text()
                .to_string()
        }
    }

    impl Backend for MarkerBackend {
        fn name(&self) -> &'static str {
            "Marker"
        }

        fn verdict(&self, program: &SubjectProgram, _limit: Duration) -> Verdict {
            if program.entry_text().contains(ReducerFixture::MARKER) {
                Verdict::Rejected(vec![ReducerFixture::rejection()])
            } else {
                Verdict::Accepted
            }
        }
    }

    impl Oracle for MarkerOracle {
        fn name(&self) -> &'static str {
            "MarkerOracle"
        }

        fn judge(&self, verdicts: &BackendVerdicts) -> Option<Finding> {
            let evidence = verdicts.of("Marker")?.rejections().to_vec();
            (!evidence.is_empty())
                .then(|| Finding::of(self.name(), verdicts, Citation::new("marked", evidence)))
        }
    }

    #[test]
    fn shrinks_to_the_smallest_text_the_oracle_still_finds() {
        let shrunk = ReducerFixture::shrunk("(defn main () (a (b)) (bad) (c))", 100);
        assert_eq!(shrunk, "(bad)\n");
    }

    #[test]
    fn keeps_a_program_the_oracle_does_not_find_and_stops_at_its_budget() {
        assert_eq!(
            ReducerFixture::shrunk("(defn main () (a))", 100),
            "(defn main () (a))"
        );
        assert_eq!(
            ReducerFixture::shrunk("(defn main () (a) (bad))", 0),
            "(defn main () (a) (bad))"
        );
    }
}
