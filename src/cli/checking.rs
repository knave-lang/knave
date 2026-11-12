use super::exit_status::ExitStatus;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use crate::check::ProgramChecker;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::expand::Program;
use crate::types::TypedProgram;

pub struct Checking<'a> {
    report: SourceReport<'a>,
    work: &'a mut StageWork,
}

impl<'a> Checking<'a> {
    pub fn new(report: SourceReport<'a>, work: &'a mut StageWork) -> Self {
        Self { report, work }
    }

    pub fn check(&mut self, program: &Program) -> Result<TypedProgram, ExitStatus> {
        self.check_showing(program, |_| true)
    }

    pub fn check_silently(&mut self, program: &Program) -> Result<TypedProgram, ExitStatus> {
        self.check_showing(program, |_| false)
    }

    pub fn check_without_ownership(
        &mut self,
        program: &Program,
    ) -> Result<TypedProgram, ExitStatus> {
        self.reported(ProgramChecker::check_without_ownership(program), |_| true)
    }

    pub fn check_showing(
        &mut self,
        program: &Program,
        shown_warning: impl FnMut(&Diagnostic) -> bool,
    ) -> Result<TypedProgram, ExitStatus> {
        self.reported(ProgramChecker::check(program), shown_warning)
    }

    fn reported(
        &mut self,
        checked: Result<TypedProgram, Diagnostics>,
        shown_warning: impl FnMut(&Diagnostic) -> bool,
    ) -> Result<TypedProgram, ExitStatus> {
        let mut typed = checked.map_err(|diagnostics| self.report.report(&diagnostics))?;
        self.work.inferred_expressions = Some(typed.checking.inferred_expressions);
        typed.warnings.retain(shown_warning);
        self.report.print(&typed.warnings);
        Ok(typed)
    }
}

#[cfg(test)]
mod tests {
    use super::Checking;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::source_report::SourceReport;
    use crate::cli::stage_work::StageWork;
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::source::SourceMap;

    struct CheckingFixture {
        sources: SourceMap,
        work: StageWork,
        output: RecordedOutput,
    }

    impl CheckingFixture {
        const TWO_WARNINGS: &str = "(enum shape (circle f64) point)\n\n(defn area (s)\n  (match s (_ 0.0) (point 1.0)))\n\n(defn add-one (items) (push! items 1))\n";

        fn new() -> Self {
            Self {
                sources: SourceMap::default(),
                work: StageWork::default(),
                output: RecordedOutput::default(),
            }
        }

        fn is_unreachable_arm(warning: &Diagnostic) -> bool {
            warning.code() == Some(&DiagnosticCode::UNREACHABLE_ARM)
        }

        fn checking(&mut self) -> Checking<'_> {
            Checking::new(
                SourceReport::new(&self.sources, &self.output),
                &mut self.work,
            )
        }
    }

    #[test]
    fn checks_a_program_and_records_its_inferred_expressions() {
        let mut fixture = CheckingFixture::new();
        let program = FakeStages::expanded(FakePrograms::WITHOUT_MAIN);
        assert!(fixture.checking().check(&program).is_ok());
        assert!(
            fixture
                .work
                .inferred_expressions
                .is_some_and(|count| count > 0)
        );
    }

    #[test]
    fn reports_a_program_that_does_not_check() {
        let mut fixture = CheckingFixture::new();
        let program = FakeStages::expanded(FakePrograms::MISTYPED);
        let status = fixture.checking().check(&program).err();
        assert_eq!(status, Some(ExitStatus::ProgramErrors));
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn prints_every_warning_of_a_checked_program() {
        let mut fixture = CheckingFixture::new();
        let program = FakeStages::expanded(FakePrograms::UNREACHABLE_ARM);
        let typed = fixture
            .checking()
            .check(&program)
            .expect("the program checks");
        assert_eq!(typed.warnings.len(), 1);
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn shows_only_the_warnings_it_is_asked_to_show() {
        let mut fixture = CheckingFixture::new();
        let program = FakeStages::expanded(CheckingFixture::TWO_WARNINGS);
        let typed = fixture
            .checking()
            .check_showing(&program, CheckingFixture::is_unreachable_arm)
            .expect("the program checks");
        let codes: Vec<&str> = typed
            .warnings
            .in_source_order()
            .into_iter()
            .filter_map(|warning| warning.code().map(DiagnosticCode::as_str))
            .collect();
        assert_eq!(codes, ["L0434"]);
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn checks_silently_without_printing_a_warning() {
        let mut fixture = CheckingFixture::new();
        let program = FakeStages::expanded(FakePrograms::UNREACHABLE_ARM);
        let typed = fixture
            .checking()
            .check_silently(&program)
            .expect("the program checks");
        assert!(typed.warnings.is_empty());
        assert!(fixture.output.writes().is_empty());
    }
}
