use crate::backend_verdicts::BackendVerdicts;
use crate::checker::Checker;
use crate::compiled_debug::CompiledDebug;
use crate::finding::Finding;
use crate::oracle::Oracle;
use crate::verdict::{Rejection, Verdict};

pub struct CheckAcceptsRustcRejects;

impl CheckAcceptsRustcRejects {
    pub const NAME: &str = "CheckAcceptsRustcRejects";
}

impl Oracle for CheckAcceptsRustcRejects {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn judge(&self, verdicts: &BackendVerdicts) -> Option<Finding> {
        let Some(Verdict::Accepted) = verdicts.of(Checker::NAME) else {
            return None;
        };
        let rustc_errors: Vec<String> = verdicts
            .of(CompiledDebug::NAME)?
            .rejections()
            .iter()
            .filter(|rejection| rejection.is_rustc() && rejection.reached_from_main)
            .map(Rejection::to_string)
            .collect();
        if rustc_errors.is_empty() {
            return None;
        }
        Some(Finding::of(
            Self::NAME,
            verdicts,
            format!(
                "check accepts and rustc rejects: {}",
                rustc_errors.join(", ")
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::CheckAcceptsRustcRejects;
    use crate::checker::Checker;
    use crate::compiled_debug::CompiledDebug;
    use crate::oracle::Oracle;
    use crate::oracle_fixture::OracleFixture;
    use crate::verdict::Verdict;

    #[test]
    fn finds_a_program_that_check_accepts_and_rustc_rejects() {
        let verdicts = OracleFixture::verdicts([
            (Checker::NAME, Verdict::Accepted),
            (
                CompiledDebug::NAME,
                OracleFixture::rejected_by_rustc("E0502"),
            ),
        ]);
        let finding = CheckAcceptsRustcRejects
            .judge(&verdicts)
            .expect("a finding");
        assert_eq!(
            finding.summary,
            "check accepts and rustc rejects: E0502 in `main`"
        );
    }

    #[test]
    fn finds_nothing_when_check_rejects_or_rustc_compiles() {
        let refused = OracleFixture::verdicts([
            (Checker::NAME, OracleFixture::rejected_by_knave("L0501")),
            (
                CompiledDebug::NAME,
                OracleFixture::rejected_by_knave("L0501"),
            ),
        ]);
        let compiled = OracleFixture::verdicts([
            (Checker::NAME, Verdict::Accepted),
            (CompiledDebug::NAME, Verdict::Accepted),
        ]);
        assert!(CheckAcceptsRustcRejects.judge(&refused).is_none());
        assert!(CheckAcceptsRustcRejects.judge(&compiled).is_none());
    }

    #[test]
    fn finds_nothing_when_knave_refuses_what_check_accepted() {
        let verdicts = OracleFixture::verdicts([
            (Checker::NAME, Verdict::Accepted),
            (
                CompiledDebug::NAME,
                OracleFixture::rejected_by_knave("L0401"),
            ),
        ]);
        let uncoded = OracleFixture::verdicts([
            (Checker::NAME, Verdict::Accepted),
            (
                CompiledDebug::NAME,
                OracleFixture::rejected_by_knave_without_a_code(),
            ),
        ]);
        assert!(CheckAcceptsRustcRejects.judge(&verdicts).is_none());
        assert!(CheckAcceptsRustcRejects.judge(&uncoded).is_none());
    }
}
