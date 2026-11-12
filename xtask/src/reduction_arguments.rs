use std::fs;
use std::path::PathBuf;

use clap::Args;
use knave_agree::{
    Finding, FindingsDirectory, Harness, ReducedFindings, ReductionPlan, ReportedSignatures,
};

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
    #[arg(
        long,
        value_name = "FILE",
        requires = "report",
        help = "Leave out of the report each finding whose signature this file, such as earlier issue comments, already holds"
    )]
    already_reported: Option<PathBuf>,
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
        let reported = self.reported()?;
        match &self.report {
            Some(path) => reduced
                .write_report(path, &reported)
                .map_err(|error| TaskFailure::unwritable(path, error)),
            None => Ok(()),
        }
    }

    fn reported(&self) -> Result<ReportedSignatures, TaskFailure> {
        match &self.already_reported {
            Some(path) => fs::read_to_string(path)
                .map(|text| ReportedSignatures::of_text(&text))
                .map_err(|error| TaskFailure::unreadable(path, error)),
            None => Ok(ReportedSignatures::default()),
        }
    }
}
