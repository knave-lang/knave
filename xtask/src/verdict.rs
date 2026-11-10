use std::process::ExitCode;

use crate::checks::Check;

pub struct Verdict;

impl Verdict {
    pub fn of(checks: &[&dyn Check]) -> ExitCode {
        let failures = checks
            .iter()
            .filter(|check| !Self::announce(**check))
            .count();
        if failures == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    fn announce(check: &dyn Check) -> bool {
        println!("{}", Self::line(check));
        check.passes()
    }

    fn line(check: &dyn Check) -> String {
        if check.passes() {
            format!("ok: {}", check.summary())
        } else {
            format!("error: {}\n{}", check.summary(), check.details())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Verdict;
    use crate::checks::Check;

    struct FixedCheck {
        passes: bool,
    }

    impl Check for FixedCheck {
        fn passes(&self) -> bool {
            self.passes
        }

        fn summary(&self) -> String {
            "the summary".to_string()
        }

        fn details(&self) -> String {
            "the details".to_string()
        }
    }

    #[test]
    fn a_passing_check_prints_its_summary() {
        assert_eq!(
            Verdict::line(&FixedCheck { passes: true }),
            "ok: the summary"
        );
    }

    #[test]
    fn a_failing_check_prints_its_summary_and_details() {
        assert_eq!(
            Verdict::line(&FixedCheck { passes: false }),
            "error: the summary\nthe details"
        );
    }
}
