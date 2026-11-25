use std::path::Path;
use std::process::ExitCode;

use clap::Args;
use knave_agree::LeftOutFixtures;

use crate::changed_functions::ChangedFunctions;
use crate::changed_lines::ChangedLines;
use crate::checks::Check;
use crate::crap_forecast::CrapForecast;
use crate::failure::TaskFailure;
use crate::git::Git;
use crate::instrumented_tests::InstrumentedTests;
use crate::test_selection::TestRun;
use crate::verdict::Verdict;

#[derive(Args)]
pub struct CrapForecastTask {
    #[arg(
        long,
        default_value = "origin/main",
        help = "Score the functions changed since the branch forked from this revision"
    )]
    base: String,
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Also run these `cargo test` arguments under coverage, such as '--test stages emit_fixtures'. CI's coverage runs no features, release builds or ignored tests, so lines that only such a run covers do not count there"
    )]
    also: Vec<String>,
}

impl CrapForecastTask {
    pub fn run(&self, root: &Path) -> ExitCode {
        self.attempt(root)
            .unwrap_or_else(|failure| failure.report())
    }

    fn attempt(&self, root: &Path) -> Result<ExitCode, TaskFailure> {
        let fork_point = Git::new(root).merge_base(&self.base)?;
        let changed = ChangedLines::since(root, &fork_point)?;
        let forecast = self.forecast(root, &changed)?;
        Ok(Verdict::announce(&[&forecast as &dyn Check]))
    }

    fn forecast(&self, root: &Path, changed: &ChangedLines) -> Result<CrapForecast, TaskFailure> {
        let functions = ChangedFunctions::find(root, changed)?;
        let runs = functions.test_runs(&self.extra_runs());
        Self::announce_left_out_fixtures(&runs);
        let report = InstrumentedTests::new(root).run_selected(&runs)?;
        Ok(functions.forecast(&report))
    }

    fn extra_runs(&self) -> Vec<TestRun> {
        self.also
            .iter()
            .map(|arguments| TestRun::parse(arguments))
            .collect()
    }

    fn announce_left_out_fixtures(runs: &[TestRun]) {
        for note in Self::left_out_fixture_notes(runs) {
            println!("note: {note}");
        }
    }

    fn left_out_fixture_notes(runs: &[TestRun]) -> Vec<String> {
        LeftOutFixtures::slow()
            .notes_for_stage_tests(|test| runs.iter().any(|run| run.selects_stage_test(test)))
    }
}

#[cfg(test)]
mod tests {
    use knave_agree::LeftOutFixtures;

    use super::CrapForecastTask;
    use crate::test_selection::TestRun;
    use crate::workspace::WorkspaceFixture;

    #[test]
    fn a_selection_with_stage_fixtures_notes_each_left_out_fixture() {
        let runs = [
            TestRun::parse("--lib emit::"),
            TestRun::parse("--test stages check_fixtures"),
        ];
        let notes = CrapForecastTask::left_out_fixture_notes(&runs);
        assert!(!notes.is_empty());
        assert_eq!(
            notes,
            LeftOutFixtures::slow().notes_for_stage_tests(|test| test == "check_fixtures")
        );
    }

    #[test]
    fn a_selection_without_stage_fixtures_notes_nothing() {
        let runs = [
            TestRun::parse("--lib emit::"),
            TestRun::parse("--test cli fmt"),
        ];
        assert!(CrapForecastTask::left_out_fixture_notes(&runs).is_empty());
    }

    #[test]
    fn a_selection_of_other_stage_fixtures_notes_nothing() {
        let runs = [
            TestRun::parse("--test stages reader_fixtures"),
            TestRun::parse("--test stages expand_fixtures"),
        ];
        assert!(CrapForecastTask::left_out_fixture_notes(&runs).is_empty());
    }

    #[test]
    fn each_also_becomes_one_extra_run() {
        let task = CrapForecastTask {
            base: "origin/main".to_string(),
            also: vec!["--test cli fmt".to_string(), "--lib".to_string()],
        };
        let runs: Vec<Vec<String>> = task
            .extra_runs()
            .iter()
            .map(|run| run.arguments().to_vec())
            .collect();
        assert_eq!(runs, [vec!["--test", "cli", "fmt"], vec!["--lib"]]);
    }

    #[test]
    fn an_unknown_base_fails_before_any_test_runs() {
        let task = CrapForecastTask {
            base: "no-such-revision".to_string(),
            also: Vec::new(),
        };
        let root = WorkspaceFixture::repository_root();
        let failure = match task.attempt(root) {
            Ok(_) => panic!("the forecast ran"),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(failure, "cannot find the revision `no-such-revision`");
    }
}
