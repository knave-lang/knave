use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Args;
use knave_agree::LeftOutFixtures;

use crate::base_revision::BaseRevision;
use crate::changed_functions::ChangedFunctions;
use crate::changed_lines::ChangedLines;
use crate::checks::Check;
use crate::crap_forecast::CrapForecast;
use crate::failure::TaskFailure;
use crate::instrumented_tests::InstrumentedTests;
use crate::test_selection::TestRun;
use crate::verdict::Verdict;

#[derive(Args)]
pub struct CrapForecastTask {
    #[command(flatten)]
    base: BaseRevision,
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Also run these `cargo test` arguments under coverage, such as '--test stages emit_fixtures'. CI's coverage runs no features, release builds or ignored tests, so lines that only such a run covers do not count there"
    )]
    also: Vec<String>,
    #[arg(
        long,
        help = "Write the verdict to this file when the forecast reaches one, and leave the file unwritten when it stops before"
    )]
    verdict: Option<PathBuf>,
}

impl CrapForecastTask {
    pub fn run(&self, root: &Path) -> ExitCode {
        self.attempt(root)
            .unwrap_or_else(|failure| failure.report())
    }

    fn attempt(&self, root: &Path) -> Result<ExitCode, TaskFailure> {
        let fork_point = self.base.fork_point(root)?;
        let changed = ChangedLines::since(root, &fork_point)?;
        let forecast = self.forecast(root, &changed)?;
        self.record_verdict(&forecast)?;
        Ok(Verdict::announce(&[&forecast as &dyn Check]))
    }

    fn record_verdict(&self, forecast: &CrapForecast) -> Result<(), TaskFailure> {
        let Some(path) = &self.verdict else {
            return Ok(());
        };
        fs::write(path, Verdict::text(&[forecast as &dyn Check]))
            .map_err(|error| TaskFailure::unwritable(path, error))
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

    use std::fs;
    use std::path::PathBuf;

    use super::CrapForecastTask;
    use crate::base_revision::BaseRevision;
    use crate::crap_forecast::CrapForecast;
    use crate::temporary_repository::TemporaryRepository;
    use crate::test_selection::TestRun;
    use crate::workspace::WorkspaceFixture;

    struct ForecastTaskCase;

    impl ForecastTaskCase {
        const VERDICT_FILE: &str = "verdict.txt";

        fn task(base: &str, verdict: Option<PathBuf>) -> CrapForecastTask {
            CrapForecastTask {
                base: BaseRevision::named(base),
                also: Vec::new(),
                verdict,
            }
        }
    }

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
            also: vec!["--test cli fmt".to_string(), "--lib".to_string()],
            ..ForecastTaskCase::task("origin/main", None)
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
        let task = ForecastTaskCase::task("no-such-revision", None);
        let root = WorkspaceFixture::repository_root();
        let failure = match task.attempt(root) {
            Ok(_) => panic!("the forecast ran"),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(failure, "cannot find the revision `no-such-revision`");
    }

    #[test]
    fn a_forecast_that_stops_writes_no_verdict_file() {
        let directory = TemporaryRepository::outside_git("stopped-forecast");
        let verdict = directory.root().join(ForecastTaskCase::VERDICT_FILE);
        let task = ForecastTaskCase::task("no-such-revision", Some(verdict.clone()));
        assert!(task.attempt(WorkspaceFixture::repository_root()).is_err());
        assert!(!verdict.exists());
    }

    #[test]
    fn a_reached_verdict_is_written_to_the_verdict_file() {
        let directory = TemporaryRepository::outside_git("reached-forecast");
        let verdict = directory.root().join(ForecastTaskCase::VERDICT_FILE);
        let task = ForecastTaskCase::task("origin/main", Some(verdict.clone()));
        task.record_verdict(&CrapForecast::new(8.0, Vec::new()))
            .expect("the verdict is written");
        assert_eq!(
            fs::read_to_string(&verdict).expect("the verdict file reads"),
            "ok: 0 changed functions scored against the CRAP limit of 8: 0 over it in CI for certain, 0 unproven\n"
        );
    }

    #[test]
    fn without_a_verdict_path_recording_writes_nothing_and_succeeds() {
        let recorded = ForecastTaskCase::task("origin/main", None)
            .record_verdict(&CrapForecast::new(8.0, Vec::new()));
        assert!(recorded.is_ok());
    }
}
