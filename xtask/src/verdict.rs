use std::process::ExitCode;

use crate::checks::Check;

pub struct Verdict;

impl Verdict {
    pub fn announce(checks: &[&dyn Check]) -> ExitCode {
        let failures = checks
            .iter()
            .filter(|check| !Self::announce_one(**check))
            .count();
        if failures == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    fn announce_one(check: &dyn Check) -> bool {
        let (passes, line) = Self::judged(check);
        println!("{line}");
        passes
    }

    fn judged(check: &dyn Check) -> (bool, String) {
        let passes = check.passes();
        let line = if passes {
            format!("ok: {}", check.summary())
        } else {
            format!("error: {}\n{}", check.summary(), check.details())
        };
        (passes, line)
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
            Verdict::judged(&FixedCheck { passes: true }),
            (true, "ok: the summary".to_string())
        );
    }

    #[test]
    fn a_failing_check_prints_its_summary_and_details() {
        assert_eq!(
            Verdict::judged(&FixedCheck { passes: false }),
            (false, "error: the summary\nthe details".to_string())
        );
    }
}
