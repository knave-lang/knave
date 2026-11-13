use crate::backend_verdicts::BackendVerdicts;
use crate::checker::Checker;
use crate::checker_without_ownership::CheckerWithoutOwnership;
use crate::citation::Citation;
use crate::finding::Finding;
use crate::oracle::Oracle;
use crate::verdict::{Rejection, Verdict};

pub struct FalseOwnershipError;

impl FalseOwnershipError {
    pub const NAME: &str = "FalseOwnershipError";
}

impl Oracle for FalseOwnershipError {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn judge(&self, verdicts: &BackendVerdicts) -> Option<Finding> {
        let ownership_errors: Vec<Rejection> = verdicts
            .of(Checker::NAME)?
            .rejections()
            .iter()
            .filter(|rejection| rejection.is_ownership() && rejection.reached_from_main)
            .cloned()
            .collect();
        // TODO(#503): any rustc error confirms a refusal, so an aliasing error masks a false
        // refusal. Match the refusal's rustc codes once the loan rule refuses aliasing in check.
        let Some(Verdict::Accepted) = verdicts.of(CheckerWithoutOwnership::NAME) else {
            return None;
        };
        if ownership_errors.is_empty() {
            return None;
        }
        Some(Finding::of(
            Self::NAME,
            verdicts,
            Citation::new("rustc compiles what check refuses", ownership_errors),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::FalseOwnershipError;
    use crate::checker::Checker;
    use crate::checker_without_ownership::CheckerWithoutOwnership;
    use crate::oracle::Oracle;
    use crate::oracle_fixture::OracleFixture;
    use crate::verdict::Verdict;

    #[test]
    fn finds_an_ownership_refusal_of_a_program_rustc_compiles() {
        let verdicts = OracleFixture::verdicts([
            (Checker::NAME, OracleFixture::rejected_by_knave("L0518")),
            (CheckerWithoutOwnership::NAME, Verdict::Accepted),
        ]);
        let finding = FalseOwnershipError.judge(&verdicts).expect("a finding");
        assert_eq!(
            finding.summary(),
            "rustc compiles what check refuses: L0518 in `main`"
        );
    }

    #[test]
    fn finds_nothing_when_rustc_agrees_or_the_refusal_is_not_about_ownership() {
        let agreed = OracleFixture::verdicts([
            (Checker::NAME, OracleFixture::rejected_by_knave("L0501")),
            (
                CheckerWithoutOwnership::NAME,
                OracleFixture::rejected_by_rustc("E0382"),
            ),
        ]);
        let mistyped = OracleFixture::verdicts([
            (Checker::NAME, OracleFixture::rejected_by_knave("L0401")),
            (CheckerWithoutOwnership::NAME, Verdict::Accepted),
        ]);
        assert!(FalseOwnershipError.judge(&agreed).is_none());
        assert!(FalseOwnershipError.judge(&mistyped).is_none());
    }

    #[test]
    fn finds_nothing_in_a_function_main_never_reaches() {
        let verdicts = OracleFixture::verdicts([
            (
                Checker::NAME,
                OracleFixture::rejected_by_knave_in_an_unreached_function("L0501"),
            ),
            (CheckerWithoutOwnership::NAME, Verdict::Accepted),
        ]);
        assert!(FalseOwnershipError.judge(&verdicts).is_none());
    }
}
