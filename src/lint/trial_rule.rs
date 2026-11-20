use super::linted_program::LintedProgram;
use super::module_checks::ModuleChecks;
use super::program_run::ProgramRun;
use super::removal::Removal;
use super::removal_trial::RemovalTrial;
use super::trial_candidate::TrialCandidate;
use crate::core::{BindingId, CoreModule};
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::types::TypedProgram;

pub trait TrialRule {
    type Declaration;
    type Kept;
    type Trial<'r, Run: ProgramRun + 'r>: RemovalTrial<Kept = Self::Kept>;

    fn candidates(&self, linted: &LintedProgram<'_>) -> Vec<Self::Declaration>;

    fn candidate<'d>(&self, declaration: &'d Self::Declaration) -> TrialCandidate<'d>;

    fn declared_in(&self, program: &Program, module: &CoreModule) -> Vec<BindingId>;

    fn trial<'r, Run: ProgramRun>(
        &self,
        run: &'r mut Run,
        program: &'r mut Program,
        declared: &'r TypedProgram,
    ) -> Self::Trial<'r, Run>;

    fn module_checks(
        &self,
        linted: &LintedProgram<'_>,
        removal: &Removal<Self::Kept>,
    ) -> ModuleChecks;

    fn warnings(
        &self,
        program: &Program,
        declarations: &[Self::Declaration],
        warned: Removal<Self::Kept>,
    ) -> Diagnostics;
}
