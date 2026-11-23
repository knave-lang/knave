use std::borrow::Borrow;

use rustc_hash::FxHashMap;

use super::by_value_signature::ByValueSignature;
use super::by_value_signatures::ByValueSignatures;
use super::declaration_edits::DeclarationEdits;
use super::declared_functions::DeclaredFunctions;
use super::function_declaration::FunctionDeclaration;
use super::program_run::ProgramRun;
use super::removal_trial::{RemovalTrial, TrialVerdict};
use super::trial_face::TrialFace;
use super::trial_program::TrialProgram;
use crate::core::{BindingId, CoreModule};
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::types::TypedProgram;

pub type ByValueRewrites = FxHashMap<BindingId, ByValueSignature>;

pub struct NeedlessTypeTrial<'r, Run: ProgramRun> {
    run: &'r mut Run,
    program: TrialProgram<'r>,
    declarations: Vec<FunctionDeclaration>,
    declared: TrialFace<&'r TypedProgram>,
}

impl<'r, Run: ProgramRun> NeedlessTypeTrial<'r, Run> {
    pub fn new(run: &'r mut Run, program: &'r mut Program, declared: &'r TypedProgram) -> Self {
        let declarations = DeclaredFunctions::new(program).candidates();
        let program = TrialProgram::new(program);
        let declared = TrialFace::new(declared, program.definitions());
        Self {
            run,
            program,
            declarations,
            declared,
        }
    }

    fn face_of(&mut self, modules: Vec<CoreModule>) -> Result<TrialFace, Diagnostics> {
        self.program
            .checked_with(modules, &mut *self.run)
            .map(|typed| TrialFace::new(typed, self.program.definitions()))
    }

    fn rewrites_for(&self, inferred: &TypedProgram, removed: &[BindingId]) -> ByValueRewrites {
        let signatures = ByValueSignatures::new(self.declared.typed(), inferred);
        self.declarations
            .iter()
            .filter(|declaration| removed.contains(&declaration.function))
            .filter_map(|declaration| {
                let rewrite = signatures.rewritten(declaration)?;
                Some((declaration.function, rewrite))
            })
            .collect()
    }

    fn by_value_verdict(
        &mut self,
        stripped: &mut TrialFace,
        rewrites: ByValueRewrites,
        removed: &[BindingId],
    ) -> TrialVerdict<ByValueRewrites> {
        let modules = DeclarationEdits::new(self.program.written()).passing_by_value(&rewrites);
        let Ok(mut expected) = self.face_of(modules) else {
            return TrialVerdict::Rejected(Vec::new());
        };
        let comparison = FaceComparison::new(&*self.run, self.program.definitions());
        comparison
            .verdict(&mut expected, stripped, removed)
            .keeping(rewrites)
    }
}

impl<Run: ProgramRun> RemovalTrial for NeedlessTypeTrial<'_, Run> {
    type Kept = ByValueRewrites;

    fn verdict(&mut self, removed: &[BindingId]) -> TrialVerdict<ByValueRewrites> {
        let modules = DeclarationEdits::new(self.program.written()).without(removed);
        let mut stripped = match self.face_of(modules) {
            Ok(stripped) => stripped,
            Err(errors) => return TrialVerdict::Rejected(self.program.holding(&errors, removed)),
        };
        let rewrites = self.rewrites_for(stripped.typed(), removed);
        if !rewrites.is_empty() {
            return self.by_value_verdict(&mut stripped, rewrites, removed);
        }
        let comparison = FaceComparison::new(&*self.run, self.program.definitions());
        comparison
            .verdict(&mut self.declared, &mut stripped, removed)
            .keeping(rewrites)
    }
}

struct FaceComparison<'a, Run: ProgramRun> {
    run: &'a Run,
    definitions: &'a [BindingId],
}

impl<'a, Run: ProgramRun> FaceComparison<'a, Run> {
    fn new(run: &'a Run, definitions: &'a [BindingId]) -> Self {
        Self { run, definitions }
    }

    fn verdict(
        &self,
        expected: &mut TrialFace<impl Borrow<TypedProgram>>,
        stripped: &mut TrialFace,
        removed: &[BindingId],
    ) -> TrialVerdict<()> {
        let changed = expected.changed_signatures(stripped, self.definitions);
        if !changed.is_empty() {
            return TrialVerdict::Rejected(
                changed
                    .into_iter()
                    .filter(|function| removed.contains(function))
                    .collect(),
            );
        }
        if expected.rust(self.run) == stripped.rust(self.run) {
            TrialVerdict::Kept(())
        } else {
            TrialVerdict::Rejected(Vec::new())
        }
    }
}
