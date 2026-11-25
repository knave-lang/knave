use super::fixture_glob::FixtureGlob;
use crate::heavy_run::{Narrowing, Selection, TestArguments};

pub struct LightStageRun {
    arguments: TestArguments,
    glob: FixtureGlob,
}

impl LightStageRun {
    pub const HEAVY_TESTS: [&str; 12] = [
        "check_fixtures",
        "emit_fixtures",
        "run_fixtures",
        "saved_prelude_check_fixtures",
        "saved_prelude_emit_fixtures",
        "saved_prelude_run_fixtures",
        "each_explanation_erroneous_example_reports_its_code_first",
        "each_explanation_example_is_laid_out",
        "each_explanation_fix_builds_and_runs",
        "each_explanation_holds_one_example_heading",
        "each_explanation_leaves_the_diagnostic_wording_to_its_problems_type",
        "each_explanation_that_describes_holds_no_runnable_example",
    ];

    pub fn from_process() -> Self {
        Self::of(
            TestArguments::from_process(),
            FixtureGlob::from_environment(),
        )
    }

    pub fn of(arguments: TestArguments, glob: FixtureGlob) -> Self {
        Self { arguments, glob }
    }

    fn selects_a_heavy_test(selection: &Selection) -> bool {
        let selects = |test: &str, filter: &str| {
            if selection.exact {
                test == filter
            } else {
                test.contains(filter)
            }
        };
        selection
            .filters
            .iter()
            .any(|filter| Self::HEAVY_TESTS.iter().any(|test| selects(test, filter)))
    }
}

impl Narrowing for LightStageRun {
    fn narrows(&self) -> bool {
        let selection = self.arguments.selection();
        !selection.filters.is_empty()
            && (self.glob.narrows() || !Self::selects_a_heavy_test(&selection))
    }
}
