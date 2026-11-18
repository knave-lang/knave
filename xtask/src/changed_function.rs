use crate::crap::ScoredFunction;
use crate::lcov::MissedLines;
use crate::outlook::Outlook;
use crate::test_selection::TestRun;

pub struct ChangedFunction {
    scored: ScoredFunction,
    missed: MissedLines,
    broader: Option<TestRun>,
}

impl ChangedFunction {
    pub fn new(scored: ScoredFunction, missed: MissedLines, broader: Option<TestRun>) -> Self {
        Self {
            scored,
            missed,
            broader,
        }
    }

    pub fn outlook(&self, maximum: f64) -> Outlook {
        if self.scored.crap <= maximum {
            Outlook::Passes
        } else if self.scored.fully_covered_score() > maximum {
            Outlook::Fails
        } else {
            Outlook::Unproven
        }
    }

    pub fn row(&self) -> String {
        self.scored.row()
    }

    pub fn unproven_advice(&self) -> String {
        if self.missed.is_empty() {
            return format!(
                "  {} scores {:.1} with no coverage record, so none of the tests prepush ran reached it. {}\n",
                self.scored.function,
                self.scored.crap,
                self.clearing_step()
            );
        }
        format!(
            "  {} scores {:.1} because the tests prepush ran miss {}. CI fails it unless another test covers those lines. {}\n",
            self.scored.function,
            self.scored.crap,
            self.missed,
            self.clearing_step()
        )
    }

    fn clearing_step(&self) -> String {
        match &self.broader {
            Some(run) => format!(
                "If `cargo test {run}` covers them, run `cargo xtask prepush --also '{run}'`; otherwise test them."
            ),
            None => "Name the test that covers them with `cargo xtask prepush --also '<cargo test arguments>'`, or test them.".to_string(),
        }
    }
}
