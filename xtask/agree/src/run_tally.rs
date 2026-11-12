use std::collections::BTreeMap;
use std::fmt;

use crate::backend_verdicts::BackendVerdicts;
use crate::form_census::FormCensus;

#[derive(Default)]
pub struct RunTally {
    programs: usize,
    outcomes: BTreeMap<TallyKey, usize>,
    codes: BTreeMap<TallyKey, usize>,
    forms: FormCensus,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TallyKey {
    backend: &'static str,
    label: String,
}

impl RunTally {
    pub fn count(&mut self, verdicts: &BackendVerdicts) {
        self.programs += 1;
        self.forms.add(&verdicts.program.forms);
        for (backend, verdict) in &verdicts.verdicts {
            Self::add(&mut self.outcomes, backend, verdict.outcome_name());
            for rejection in verdict.rejections() {
                Self::add(&mut self.codes, backend, rejection.code_text());
            }
        }
    }

    fn add(counts: &mut BTreeMap<TallyKey, usize>, backend: &'static str, label: &str) {
        let key = TallyKey {
            backend,
            label: label.to_string(),
        };
        *counts.entry(key).or_default() += 1;
    }
}

impl fmt::Display for TallyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.backend, self.label)
    }
}

impl fmt::Display for RunTally {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "{} programs", self.programs)?;
        for (key, count) in self.outcomes.iter().chain(&self.codes) {
            writeln!(formatter, "  {key}: {count}")?;
        }
        write!(formatter, "{}", self.forms)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use knave_expand::diagnostic::DiagnosticCode;

    use super::RunTally;
    use crate::backend_verdicts::BackendVerdicts;
    use crate::subject_program::SubjectProgram;
    use crate::verdict::{RejectingTool, Rejection, Verdict};

    struct TallyFixture;

    impl TallyFixture {
        fn verdicts(checker: Verdict) -> BackendVerdicts {
            BackendVerdicts {
                program: SubjectProgram::of_entry(
                    "probe".to_string(),
                    PathBuf::from("probe.knv"),
                    "(defn main () (println 1))".to_string(),
                ),
                verdicts: BTreeMap::from([("Checker", checker)]),
            }
        }

        fn refused() -> Verdict {
            Verdict::Rejected(vec![Rejection {
                tool: RejectingTool::Knave,
                code: Some(DiagnosticCode::from_text("L0501".to_string())),
                file: PathBuf::from("probe.knv"),
                span: 0..0,
                function: Some("main".to_string()),
                reached_from_main: true,
            }])
        }
    }

    #[test]
    fn counts_programs_outcomes_codes_and_forms() {
        let mut tally = RunTally::default();
        tally.count(&TallyFixture::verdicts(Verdict::Accepted));
        tally.count(&TallyFixture::verdicts(TallyFixture::refused()));
        tally.count(&TallyFixture::verdicts(TallyFixture::refused()));
        assert_eq!(
            tally.to_string(),
            "3 programs\n  Checker accepted: 1\n  Checker rejected: 2\n  Checker L0501: 2\n  kernel forms: \n  prelude names: defn println\n"
        );
    }
}
