use std::mem;

use super::declaration_edits::DeclarationEdits;
use super::linted_program::LintedProgram;
use super::needless_derive_rule::NeedlessDeriveRule;
use super::needless_type_rule::NeedlessTypeRule;
use super::program_run::ProgramRun;
use super::trial_levels::TrialLevels;
use super::trial_lint::TrialLint;
use crate::core::{BindingId, CoreModule};
use crate::diagnostic::Diagnostics;
use crate::project::LintLevel;
use crate::types::TypedProgram;

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
        let types = TrialLint::new(&mut *self.run, &NeedlessTypeRule)
            .outcome(levels.needless_type, programs);
        let mut warnings = types.warnings;
        if levels.needless_derive != Some(LintLevel::Allow) {
            warnings.extend(self.derives_without_types(levels, programs, &types.removed));
        }
        warnings
    }

    fn derives_without_types(
        &mut self,
        levels: TrialLevels,
        programs: &mut [LintedProgram<'_>],
        removed: &[Vec<BindingId>],
    ) -> Diagnostics {
        let written: Vec<Option<Vec<CoreModule>>> = programs
            .iter_mut()
            .zip(removed)
            .map(|(linted, removed)| Self::strip_types(linted, removed))
            .collect();
        let warnings = self.derive_warnings(levels.needless_derive, programs, &written);
        for (linted, written) in programs.iter_mut().zip(written) {
            if let Some(modules) = written {
                linted.program.modules = modules;
            }
        }
        warnings
    }

    fn strip_types(
        linted: &mut LintedProgram<'_>,
        removed: &[BindingId],
    ) -> Option<Vec<CoreModule>> {
        if removed.is_empty() {
            return None;
        }
        let stripped = DeclarationEdits::new(&linted.program.modules).without(removed);
        Some(mem::replace(&mut linted.program.modules, stripped))
    }

    fn derive_warnings(
        &mut self,
        level: Option<LintLevel>,
        programs: &mut [LintedProgram<'_>],
        written: &[Option<Vec<CoreModule>>],
    ) -> Diagnostics {
        let rechecked: Option<Vec<Option<TypedProgram>>> = programs
            .iter()
            .zip(written)
            .map(|(linted, written)| match written {
                None => Some(None),
                Some(_) => self.run.check(linted.program).ok().map(Some),
            })
            .collect();
        let Some(rechecked) = rechecked else {
            return Diagnostics::default();
        };
        let mut linted: Vec<LintedProgram<'_>> = programs
            .iter_mut()
            .zip(&rechecked)
            .map(|(linted, rechecked)| LintedProgram {
                program: &mut *linted.program,
                declared: rechecked.as_ref().unwrap_or(linted.declared),
                holds_unlinted_example: linted.holds_unlinted_example,
            })
            .collect();
        TrialLint::new(&mut *self.run, &self.derive)
            .outcome(level, &mut linted)
            .warnings
    }
}
