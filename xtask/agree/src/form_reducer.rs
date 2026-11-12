use crate::finding_probe::FindingProbe;
use crate::finding_signature::FindingSignature;
use crate::reducer::Reducer;
use crate::reduction_candidates::ReductionCandidates;
use crate::subject_program::SubjectProgram;

pub struct FormReducer {
    attempts: usize,
}

struct Reduction<'a> {
    probe: &'a FindingProbe<'a>,
    target: FindingSignature,
    attempts_left: usize,
}

struct Candidate {
    program: SubjectProgram,
    position: usize,
}

impl FormReducer {
    pub fn new(attempts: usize) -> Self {
        Self { attempts }
    }
}

impl Reducer for FormReducer {
    fn shrink(&self, program: SubjectProgram, probe: &FindingProbe) -> SubjectProgram {
        let Some(target) = probe.signature(&program) else {
            return program;
        };
        let mut reduction = Reduction {
            probe,
            target,
            attempts_left: self.attempts,
        };
        let mut smallest = program;
        let mut start = 0;
        loop {
            match reduction.smaller(&smallest, start) {
                Some(candidate) => {
                    smallest = candidate.program;
                    start = candidate.position;
                }
                None if start > 0 => start = 0,
                None => break,
            }
        }
        smallest
    }
}

impl Reduction<'_> {
    fn smaller(&mut self, program: &SubjectProgram, start: usize) -> Option<Candidate> {
        let candidates = ReductionCandidates::of(program.entry_text());
        for (position, text) in candidates.texts().enumerate().skip(start) {
            self.attempts_left = self.attempts_left.checked_sub(1)?;
            let candidate = program.with_entry_text(text);
            if self.probe.signature(&candidate).as_ref() == Some(&self.target) {
                return Some(Candidate {
                    program: candidate,
                    position,
                });
            }
        }
        None
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
    use crate::finding_probe::FindingProbe;
    use crate::oracle::Oracle;
    use crate::reducer::Reducer;
    use crate::subject_program::SubjectProgram;
    use crate::verdict::{RejectingTool, Rejection, Verdict};

    struct MarkerBackend;

    struct MarkerOracle;

    struct ReducerFixture;

    impl ReducerFixture {
        const MARKER: &str = "(bad)";
        const ENTRY: &str = "(main";
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
            let probe = FindingProbe::new(&MarkerOracle, vec![&MarkerBackend], Self::LIMIT);
            FormReducer::new(attempts)
                .shrink(Self::program(text), &probe)
                .entry_text()
                .to_string()
        }
    }

    impl Backend for MarkerBackend {
        fn name(&self) -> &'static str {
            "Marker"
        }

        fn verdict(&self, program: &SubjectProgram, _limit: Duration) -> Verdict {
            let text = program.entry_text();
            if text.starts_with(ReducerFixture::ENTRY) && text.contains(ReducerFixture::MARKER) {
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
        let shrunk = ReducerFixture::shrunk("(main (a (b)) (bad) (c))", 100);
        assert_eq!(shrunk, "(main  (bad) )\n");
    }

    #[test]
    fn starts_over_from_the_largest_list_when_the_later_candidates_find_nothing() {
        let shrunk = ReducerFixture::shrunk("(main (a) (bad) (a))", 100);
        assert_eq!(shrunk, "(main  (bad) )\n");
    }

    #[test]
    fn keeps_a_program_the_oracle_does_not_find_and_stops_at_its_budget() {
        assert_eq!(ReducerFixture::shrunk("(main (a))", 100), "(main (a))");
        assert_eq!(
            ReducerFixture::shrunk("(main (a) (bad))", 0),
            "(main (a) (bad))"
        );
    }
}
