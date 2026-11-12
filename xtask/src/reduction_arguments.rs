use std::path::PathBuf;

use clap::Args;
use knave_agree::{Finding, FindingsDirectory, Harness, ReducedFindings, ReductionPlan};

use crate::failure::TaskFailure;

#[derive(Args)]
pub struct ReductionArguments {
    #[arg(
        long = "reduce",
        value_name = "COUNT",
        default_value_t = 0,
        help = "Shrink up to this many findings, those outside the known classes first, and write each as `<seed>.reduced.knv`"
    )]
    count: usize,
    #[arg(
        long,
        value_name = "FILE",
        help = "Write the reduced findings outside the known classes to this Markdown file, if there are any"
    )]
    report: Option<PathBuf>,
}

impl ReductionArguments {
    const ATTEMPTS: usize = 400;

    pub fn run(
        &self,
        harness: &Harness,
        findings: &[Finding],
        directory: &FindingsDirectory,
    ) -> Result<(), TaskFailure> {
        let reduced = ReductionPlan::before_the_loan_rule(Self::ATTEMPTS, self.count)
            .reduce(harness, findings);
        reduced
            .record(directory.path())
            .map_err(|error| TaskFailure::unwritable(directory.path(), error))?;
        self.write_report(&reduced)
    }

    fn write_report(&self, reduced: &ReducedFindings) -> Result<(), TaskFailure> {
        match &self.report {
            Some(path) => reduced
                .write_report(path)
                .map_err(|error| TaskFailure::unwritable(path, error)),
            None => Ok(()),
        }
    }
}
