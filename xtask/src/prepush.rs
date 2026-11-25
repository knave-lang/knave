use std::path::Path;
use std::process::ExitCode;

use clap::Args;

use crate::base_revision::BaseRevision;
use crate::checks::Check;
use crate::design::DesignRules;
use crate::failure::TaskFailure;
use crate::production_lines::ProductionLineCensus;
use crate::ratchet_gate::RatchetGate;
use crate::source_format::SourceFormat;
use crate::verdict::Verdict;

#[derive(Args)]
pub struct PrepushTask {
    #[command(flatten)]
    base: BaseRevision,
}

impl PrepushTask {
    const MEASURED_TREE: &str = "prepush measures the working tree, uncommitted edits included. Git's diff leaves out untracked files, so add new files to the index first.";
    const FORECAST_ON_GITHUB: &str = "prepush does not run the CRAP forecast. After this push, dispatch the `CRAP forecast` workflow if the push adds or changes a function, answers a forecast verdict, or comes before `gh pr ready`, as the CRAP forecast section of docs/pull-requests.md says.";

    pub fn run(&self, root: &Path) -> ExitCode {
        let verdict = self
            .attempt(root)
            .unwrap_or_else(|failure| failure.report());
        for line in Self::closing_lines(verdict) {
            println!("{line}");
        }
        verdict
    }

    fn closing_lines(verdict: ExitCode) -> Vec<&'static str> {
        let mut lines = vec![Self::MEASURED_TREE];
        if verdict == ExitCode::SUCCESS {
            lines.push(Self::FORECAST_ON_GITHUB);
        }
        lines
    }

    fn attempt(&self, root: &Path) -> Result<ExitCode, TaskFailure> {
        let fork_point = self.base.fork_point(root)?;
        let ratchets = Self::ratchet_checks(root, &fork_point)?;
        let checks = Self::checks(SourceFormat::check(root)?, ratchets);
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Verdict::announce(&checks))
    }

    fn checks(format: SourceFormat, ratchets: Vec<Box<dyn Check>>) -> Vec<Box<dyn Check>> {
        let mut checks: Vec<Box<dyn Check>> = vec![Box::new(format)];
        checks.extend(ratchets);
        checks
    }

    fn ratchet_checks(root: &Path, fork_point: &str) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        RatchetGate::new(root, fork_point).checks(&[&DesignRules, &ProductionLineCensus])
    }
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::PrepushTask;
    use crate::base_revision::BaseRevision;
    use crate::checks::Check;
    use crate::source_format::{FormatOutputs, SourceFormat};
    use crate::workspace::WorkspaceFixture;

    struct PrepushCase;

    impl PrepushCase {
        fn summaries(checks: &[Box<dyn Check>]) -> Vec<String> {
            checks.iter().map(|check| check.summary()).collect()
        }
    }

    #[test]
    fn prepush_runs_the_layout_and_each_ratchet() {
        let format = SourceFormat::finished(1, FormatOutputs::passed());
        let ratchet = Box::new(SourceFormat::finished(2, FormatOutputs::passed()));
        let summaries = PrepushCase::summaries(&PrepushTask::checks(format, vec![ratchet]));
        assert_eq!(summaries.len(), 2, "{summaries:?}");
        assert!(
            summaries[0].starts_with("1 source files"),
            "{}",
            summaries[0]
        );
        assert!(
            summaries[1].starts_with("2 source files"),
            "{}",
            summaries[1]
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
    fn a_passing_prepush_points_at_the_forecast_workflow() {
        assert_eq!(
            PrepushTask::closing_lines(ExitCode::SUCCESS),
            [PrepushTask::MEASURED_TREE, PrepushTask::FORECAST_ON_GITHUB]
        );
    }

    #[test]
    fn a_failing_prepush_names_no_dispatch() {
        assert_eq!(
            PrepushTask::closing_lines(ExitCode::FAILURE),
            [PrepushTask::MEASURED_TREE]
        );
    }

    #[test]
    fn an_unknown_base_fails_before_any_check() {
        let task = PrepushTask {
            base: BaseRevision::named("no-such-revision"),
        };
        let failure = match task.attempt(WorkspaceFixture::repository_root()) {
            Ok(_) => panic!("prepush ran its checks"),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(failure, "cannot find the revision `no-such-revision`");
    }
}
