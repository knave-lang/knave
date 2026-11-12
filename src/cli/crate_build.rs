use super::build_route::BuildRoute;
use super::exit_status::ExitStatus;
use super::interruption::Interruption;
use super::source_report::SourceReport;
use super::standard_error::StandardError;
use crate::build::{BuildError, BuildProfile};
use crate::emit::EmittedCrate;

pub struct CrateBuild<'a> {
    report: SourceReport<'a>,
    interruption: &'a Interruption,
}

impl<'a> CrateBuild<'a> {
    pub fn new(report: SourceReport<'a>, interruption: &'a Interruption) -> Self {
        Self {
            report,
            interruption,
        }
    }

    pub fn compile(
        &self,
        emitted: &EmittedCrate,
        route: &dyn BuildRoute,
        profile: BuildProfile,
    ) -> Result<(), ExitStatus> {
        if self.interruption.is_raised() {
            return Err(self.report_build(BuildError::Interrupted));
        }
        route
            .compile(emitted, profile, &self.report)
            .map_err(|error| self.report_build(self.interrupted_or(error)))
    }

    fn interrupted_or(&self, error: BuildError) -> BuildError {
        if self.interruption.caused_a_failure() {
            return BuildError::Interrupted;
        }
        error
    }

    fn report_build(&self, error: BuildError) -> ExitStatus {
        match error {
            BuildError::Program(diagnostics) => self.report.report(&diagnostics),
            BuildError::Environment(message) => {
                StandardError::write_line(format_args!("error: {message}"));
                ExitStatus::Environment
            }
            BuildError::Interrupted => ExitStatus::Interrupted,
        }
    }
}
