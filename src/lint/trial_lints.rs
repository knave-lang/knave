use super::derive_declaration::DeriveDeclaration;
use super::linted_program::LintedProgram;
use super::needless_derive_rule::NeedlessDeriveRule;
use super::needless_type_rule::NeedlessTypeRule;
use super::program_candidates::ProgramCandidates;
use super::program_run::ProgramRun;
use super::stripped_programs::StrippedPrograms;
use super::trial_levels::TrialLevels;
use super::trial_lint::TrialLint;
use super::unlinted_examples::UnlintedExamples;
use crate::diagnostic::Diagnostics;
pub struct TrialLints<'r, Run: ProgramRun> {
    run: &'r mut Run,
    derive: NeedlessDeriveRule,
}

impl<'r, Run: ProgramRun> TrialLints<'r, Run> {
    pub fn new(run: &'r mut Run, derive: NeedlessDeriveRule) -> Self {
        Self { run, derive }
    }

    pub fn warnings(
        &mut self,
        levels: TrialLevels,
        programs: &mut [LintedProgram<'_>],
    ) -> Diagnostics {
        let mut types = TrialLint::new(&mut *self.run, &NeedlessTypeRule);
        let candidates = types.candidates(levels.needless_type, programs);
        let types = types.outcome(candidates, programs);
        let mut warnings = types.warnings;
        let derives = TrialLint::new(&mut *self.run, &self.derive)
            .candidates(levels.needless_derive, programs);
        if !derives.tries_any() {
            return warnings;
        }
        UnlintedExamples::new(&mut *self.run).mark(&derives, programs);
        let stripped = StrippedPrograms::strip(programs, &types.removed);
        warnings.extend(self.derive_warnings(derives, &stripped, programs));
        stripped.restore(programs);
        warnings
    }

    fn derive_warnings(
        &mut self,
        derives: ProgramCandidates<DeriveDeclaration>,
        stripped: &StrippedPrograms,
        programs: &mut [LintedProgram<'_>],
    ) -> Diagnostics {
        let Some(rechecked) = stripped.rechecked(programs, &mut *self.run) else {
            return Diagnostics::default();
        };
        let mut linted: Vec<LintedProgram<'_>> = programs
            .iter_mut()
            .zip(&rechecked)
            .map(|(linted, rechecked)| LintedProgram {
                program: &mut *linted.program,
                declared: rechecked.as_ref().unwrap_or(linted.declared),
                examples: linted.examples,
                holds_unlinted_example: linted.holds_unlinted_example,
            })
            .collect();
        TrialLint::new(&mut *self.run, &self.derive)
            .outcome(derives, &mut linted)
            .warnings
    }
}
