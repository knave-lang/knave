use super::fixture_glob::FixtureGlob;
use crate::heavy_run::{Narrowing, Selection, TestArguments};

pub struct StageRun {
    arguments: TestArguments,
    glob: FixtureGlob,
}

impl StageRun {
    pub const FIXTURE_TESTS: [&str; 6] = [
        "check_fixtures",
        "emit_fixtures",
        "run_fixtures",
        "saved_prelude_check_fixtures",
        "saved_prelude_emit_fixtures",
        "saved_prelude_run_fixtures",
    ];
    pub const EXPLANATION_TESTS: [&str; 6] = [
        "each_explanation_erroneous_example_reports_its_code_first",
        "each_explanation_example_is_laid_out",
        "each_explanation_fix_builds_and_runs",
        "each_explanation_holds_one_example_heading",
        "each_explanation_leaves_the_diagnostic_wording_to_its_problems_type",
        "each_explanation_that_describes_holds_no_runnable_example",
    ];
    const HEAVY_FIXTURE_SUFFIXES: [&str; 3] = ["check_fixtures", "run_fixtures", "emit_fixtures"];
    const EXPLANATION_PREFIX: &str = "each_explanation_";
    const FIXTURE_TEST_SUFFIX: &str = "_fixtures";

    pub fn from_process() -> Self {
        Self::of(
            TestArguments::from_process(),
            FixtureGlob::from_environment(),
        )
    }

    pub fn of(arguments: TestArguments, glob: FixtureGlob) -> Self {
        Self { arguments, glob }
    }

    pub fn heavy_tests() -> Vec<&'static str> {
        [Self::FIXTURE_TESTS, Self::EXPLANATION_TESTS].concat()
    }

    pub fn is_heavy_name(test: &str) -> bool {
        Self::HEAVY_FIXTURE_SUFFIXES
            .iter()
            .any(|suffix| test.ends_with(suffix))
            || test.starts_with(Self::EXPLANATION_PREFIX)
    }

    pub fn is_guarded_name(test: &str) -> bool {
        test.ends_with(Self::FIXTURE_TEST_SUFFIX) || test.starts_with(Self::EXPLANATION_PREFIX)
    }

    fn selects_any(selection: &Selection, tests: &[&str]) -> bool {
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
            .any(|filter| tests.iter().any(|test| selects(test, filter)))
    }
}

impl Narrowing for StageRun {
    fn narrows(&self) -> bool {
        let selection = self.arguments.selection();
        let names_one_filter_or_exact_tests = selection.filters.len() == 1 || selection.exact;
        !selection.filters.is_empty()
            && names_one_filter_or_exact_tests
            && !Self::selects_any(&selection, &Self::EXPLANATION_TESTS)
            && (self.glob.narrows() || !Self::selects_any(&selection, &Self::FIXTURE_TESTS))
    }
}
