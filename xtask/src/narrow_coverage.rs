use std::fs;
use std::path::Path;
use std::process::Command;

use crate::cargo::Cargo;
use crate::failure::TaskFailure;
use crate::lcov::LcovReport;
use crate::test_selection::TestRun;

pub struct NarrowCoverage<'a> {
    root: &'a Path,
}

impl<'a> NarrowCoverage<'a> {
    const REPORT: &'static str = "target/coverage/prepush.info";
    const FORGET_PREVIOUS_RUNS: [&'static str; 3] = ["llvm-cov", "clean", "--profraw-only"];
    const RUN_WITHOUT_REPORT: [&'static str; 3] = ["llvm-cov", "test", "--no-report"];
    const WRITE_REPORT: [&'static str; 3] = ["llvm-cov", "report", "--lcov"];

    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub fn measure(&self, runs: &[TestRun]) -> Result<LcovReport, TaskFailure> {
        if runs.is_empty() {
            return Ok(LcovReport::parse("", self.root));
        }
        self.cargo(|command| command.args(Self::FORGET_PREVIOUS_RUNS))?;
        for run in runs {
            self.cargo(|command| command.args(Self::RUN_WITHOUT_REPORT).args(run.arguments()))?;
        }
        self.report()
    }

    fn report(&self) -> Result<LcovReport, TaskFailure> {
        let report = self.root.join(Self::REPORT);
        Self::create_parent(&report)?;
        self.cargo(|command| {
            command
                .args(Self::WRITE_REPORT)
                .arg("--output-path")
                .arg(&report)
        })?;
        let text =
            fs::read_to_string(&report).map_err(|error| TaskFailure::unreadable(&report, error))?;
        Ok(LcovReport::parse(&text, self.root))
    }

    fn create_parent(report: &Path) -> Result<(), TaskFailure> {
        let directory = report.parent().unwrap_or(report);
        fs::create_dir_all(directory).map_err(|error| TaskFailure::unwritable(directory, error))
    }

    fn cargo(
        &self,
        arguments: impl FnOnce(&mut Command) -> &mut Command,
    ) -> Result<(), TaskFailure> {
        let mut command = Cargo::command();
        command.current_dir(self.root);
        Cargo::run(arguments(&mut command), |stderr| {
            TaskFailure::tool_failed("cargo llvm-cov", stderr)
        })
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::NarrowCoverage;

    #[test]
    fn no_runs_measure_nothing_and_run_no_cargo() {
        let coverage = NarrowCoverage::new(Path::new("/nonexistent/workspace"));
        let report = coverage.measure(&[]).expect("nothing runs");
        assert_eq!(report.total().percent(), 0.0);
    }
}
