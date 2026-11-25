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

    pub fn text(checks: &[&dyn Check]) -> String {
        checks
            .iter()
            .map(|check| format!("{}\n", Self::judged(*check).1))
            .collect()
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

    #[test]
    fn the_text_holds_one_judgment_per_check() {
        let passing = FixedCheck { passes: true };
        let failing = FixedCheck { passes: false };
        assert_eq!(
            Verdict::text(&[&passing, &failing]),
            "ok: the summary\nerror: the summary\nthe details\n"
        );
    }
}
