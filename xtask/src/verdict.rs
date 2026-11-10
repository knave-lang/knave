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
        let passes = check.passes();
        if passes {
            println!("ok: {}", check.summary());
        } else {
            println!("error: {}\n{}", check.summary(), check.details());
        }
        passes
    }
}
