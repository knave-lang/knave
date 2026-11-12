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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::CrateBuild;
    use crate::build::{BuildError, BuildProfile};
    use crate::check::ProgramChecker;
    use crate::cli::build_route::BuildRoute;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::interruption::Interruption;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::source_report::SourceReport;
    use crate::emit::EmittedCrate;
    use crate::source::SourceMap;

    struct FakeRoute {
        failure: Option<fn() -> BuildError>,
        compiles: Cell<usize>,
    }

    impl FakeRoute {
        fn compiling() -> Self {
            Self {
                failure: None,
                compiles: Cell::new(0),
            }
        }

        fn failing_with(failure: fn() -> BuildError) -> Self {
            Self {
                failure: Some(failure),
                compiles: Cell::new(0),
            }
        }
    }

    impl BuildRoute for FakeRoute {
        fn compile(
            &self,
            _emitted: &EmittedCrate,
            _profile: BuildProfile,
            _report: &SourceReport<'_>,
        ) -> Result<(), BuildError> {
            self.compiles.set(self.compiles.get() + 1);
            self.failure.map_or(Ok(()), |failure| Err(failure()))
        }
    }

    struct BuildFixture {
        sources: SourceMap,
        output: RecordedOutput,
        interruption: Interruption,
    }

    impl BuildFixture {
        fn new(interruption: Interruption) -> Self {
            Self {
                sources: SourceMap::default(),
                output: RecordedOutput::default(),
                interruption,
            }
        }

        fn compile(&self, route: &FakeRoute) -> Result<(), ExitStatus> {
            let emitted = FakeStages::emitted(FakePrograms::PRINTS_A_SUM);
            CrateBuild::new(
                SourceReport::new(&self.sources, &self.output),
                &self.interruption,
            )
            .compile(&emitted, route, BuildProfile::Debug)
        }

        fn program_error() -> BuildError {
            let program = FakeStages::expanded(FakePrograms::MISTYPED);
            let errors = ProgramChecker::check(&program)
                .err()
                .expect("the mistyped program fails to check");
            BuildError::Program(errors)
        }

        fn environment_error() -> BuildError {
            BuildError::Environment("no rustc".to_string())
        }
    }

    #[test]
    fn compiles_through_its_route() {
        let fixture = BuildFixture::new(Interruption::default());
        let route = FakeRoute::compiling();
        assert_eq!(fixture.compile(&route), Ok(()));
        assert_eq!(route.compiles.get(), 1);
    }

    #[test]
    fn stops_before_the_route_once_interrupted() {
        let fixture = BuildFixture::new(Interruption::raised());
        let route = FakeRoute::compiling();
        assert_eq!(fixture.compile(&route), Err(ExitStatus::Interrupted));
        assert_eq!(route.compiles.get(), 0);
    }

    #[test]
    fn reports_the_program_errors_of_a_failed_route() {
        let fixture = BuildFixture::new(Interruption::default());
        let route = FakeRoute::failing_with(BuildFixture::program_error);
        assert_eq!(fixture.compile(&route), Err(ExitStatus::ProgramErrors));
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn reports_an_environment_failure_of_a_route() {
        let fixture = BuildFixture::new(Interruption::default());
        let route = FakeRoute::failing_with(BuildFixture::environment_error);
        assert_eq!(fixture.compile(&route), Err(ExitStatus::Environment));
        assert!(fixture.output.writes().is_empty());
    }

    #[test]
    fn reports_an_interrupted_route() {
        let fixture = BuildFixture::new(Interruption::default());
        let route = FakeRoute::failing_with(|| BuildError::Interrupted);
        assert_eq!(fixture.compile(&route), Err(ExitStatus::Interrupted));
    }
}
