use std::path::Path;
use std::process::ExitCode;

use clap::Args;

use crate::changed_functions::ChangedFunctions;
use crate::changed_lines::ChangedLines;
use crate::checks::Check;
use crate::crap_forecast::CrapForecast;
use crate::design::DesignRules;
use crate::failure::TaskFailure;
use crate::git::Git;
use crate::instrumented_tests::InstrumentedTests;
use crate::production_lines::ProductionLineCensus;
use crate::ratchet_gate::RatchetGate;
use crate::source_format::SourceFormat;
use crate::test_selection::TestRun;
use crate::verdict::Verdict;

#[derive(Args)]
pub struct PrepushTask {
    #[arg(
        long,
        default_value = "origin/main",
        help = "Check the changes since this branch forked from this revision"
    )]
    base: String,
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Also run these `cargo test` arguments under coverage, such as '--test stages emit_fixtures'. CI's coverage runs no features, release builds or ignored tests, so lines that only such a run covers do not count there"
    )]
    also: Vec<String>,
}

impl PrepushTask {
    const MEASURED_TREE: &str = "prepush measures the working tree, uncommitted edits included. Git's diff leaves out untracked files, so add new files to the index first.";

    pub fn run(&self, root: &Path) -> ExitCode {
        let verdict = self
            .attempt(root)
            .unwrap_or_else(|failure| failure.report());
        println!("{}", Self::MEASURED_TREE);
        verdict
    }

    fn attempt(&self, root: &Path) -> Result<ExitCode, TaskFailure> {
        let fork_point = self.fork_point(root)?;
        let changed = ChangedLines::since(root, &fork_point)?;
        let checks = Self::checks(
            self.crap_forecast(root, &changed)?,
            SourceFormat::check(root)?,
            Self::ratchet_checks(root, &fork_point)?,
        );
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Verdict::announce(&checks))
    }

    fn checks(
        forecast: CrapForecast,
        format: SourceFormat,
        ratchets: Vec<Box<dyn Check>>,
    ) -> Vec<Box<dyn Check>> {
        let mut checks: Vec<Box<dyn Check>> = vec![Box::new(forecast), Box::new(format)];
        checks.extend(ratchets);
        checks
    }

    fn fork_point(&self, root: &Path) -> Result<String, TaskFailure> {
        Git::new(root).merge_base(&self.base)
    }

    fn crap_forecast(
        &self,
        root: &Path,
        changed: &ChangedLines,
    ) -> Result<CrapForecast, TaskFailure> {
        let functions = ChangedFunctions::find(root, changed)?;
        let also: Vec<TestRun> = self
            .also
            .iter()
            .map(|arguments| TestRun::parse(arguments))
            .collect();
        let report = InstrumentedTests::new(root).run_selected(&functions.test_runs(&also))?;
        Ok(functions.forecast(&report))
    }

    fn ratchet_checks(root: &Path, fork_point: &str) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        RatchetGate::new(root, fork_point).checks(&[&DesignRules, &ProductionLineCensus])
    }
}

#[cfg(test)]
mod tests {
    use super::PrepushTask;
    use crate::checks::Check;
    use crate::crap_forecast::CrapForecast;
    use crate::source_format::{FormatOutputs, SourceFormat};
    use crate::workspace::WorkspaceFixture;

    struct PrepushCase;

    impl PrepushCase {
        fn summaries(checks: &[Box<dyn Check>]) -> Vec<String> {
            checks.iter().map(|check| check.summary()).collect()
        }
    }

    #[test]
    fn prepush_runs_the_forecast_the_layout_and_each_ratchet() {
        let forecast = CrapForecast::new(8.0, Vec::new());
        let format = SourceFormat::finished(1, FormatOutputs::passed());
        let ratchet = Box::new(SourceFormat::finished(2, FormatOutputs::passed()));
        let summaries =
            PrepushCase::summaries(&PrepushTask::checks(forecast, format, vec![ratchet]));
        assert_eq!(summaries.len(), 3, "{summaries:?}");
        assert!(summaries[0].contains("CRAP"), "{}", summaries[0]);
        assert!(
            summaries[1].starts_with("1 source files"),
            "{}",
            summaries[1]
        );
        assert!(
            summaries[2].starts_with("2 source files"),
            "{}",
            summaries[2]
        );
    }

    #[test]
    fn the_ratchets_hold_file_sizes_and_refuse_raises_since_the_fork_point() {
        let root = WorkspaceFixture::repository_root();
        let checks = PrepushTask::ratchet_checks(root, "HEAD").expect("the ratchets read");
        let summaries = PrepushCase::summaries(&checks);
        assert!(
            summaries
                .iter()
                .any(|summary| summary.contains("production lines")),
            "{summaries:?}"
        );
        assert!(
            summaries.iter().any(|summary| summary.contains("methods")),
            "{summaries:?}"
        );
    }

    #[test]
    fn an_unknown_base_fails_before_any_check() {
        let task = PrepushTask {
            base: "no-such-revision".to_string(),
            also: Vec::new(),
        };
        let failure = match task.fork_point(WorkspaceFixture::repository_root()) {
            Ok(fork_point) => panic!("found a fork point {fork_point}"),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(failure, "cannot find the revision `no-such-revision`");
    }
}
