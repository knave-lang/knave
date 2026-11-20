use std::borrow::Borrow;

use super::program_run::ProgramRun;
use crate::core::BindingId;
use crate::types::TypedProgram;

pub struct TrialFace<Typed: Borrow<TypedProgram> = TypedProgram> {
    typed: Typed,
    signatures: Vec<Option<String>>,
    rust: Option<Option<String>>,
}

impl<Typed: Borrow<TypedProgram>> TrialFace<Typed> {
    pub fn new(typed: Typed, definitions: &[BindingId]) -> Self {
        let signatures = definitions
            .iter()
            .map(|binding| typed.borrow().signature(*binding))
            .collect();
        Self {
            typed,
            signatures,
            rust: None,
        }
    }

    pub fn typed(&self) -> &TypedProgram {
        self.typed.borrow()
    }

    pub fn changed_signatures<Other: Borrow<TypedProgram>>(
        &self,
        other: &TrialFace<Other>,
        definitions: &[BindingId],
    ) -> Vec<BindingId> {
        definitions
            .iter()
            .zip(self.signatures.iter().zip(&other.signatures))
            .filter(|(_, (mine, theirs))| mine != theirs)
            .map(|(binding, _)| *binding)
            .collect()
    }

    pub fn rust(&mut self, run: &impl ProgramRun) -> &Option<String> {
        let typed = self.typed.borrow();
        self.rust.get_or_insert_with(|| run.emit(typed))
    }
}
