use super::exit_status::ExitStatus;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use crate::check::ProgramChecker;
use crate::diagnostic::Diagnostic;
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

    pub fn check_showing(
        &mut self,
        program: &Program,
        shown_warning: impl FnMut(&Diagnostic) -> bool,
    ) -> Result<TypedProgram, ExitStatus> {
        let mut typed = ProgramChecker::check(program)
            .map_err(|diagnostics| self.report.report(&diagnostics))?;
        self.work.inferred_expressions = Some(typed.checking.inferred_expressions);
        typed.warnings.retain(shown_warning);
        self.report.print(&typed.warnings);
        Ok(typed)
    }
}
