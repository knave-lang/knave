use super::program_run::ProgramRun;
use crate::core::BindingId;
use crate::types::TypedProgram;

pub struct TrialFace {
    typed: TypedProgram,
    signatures: Vec<Option<String>>,
    rust: Option<Option<String>>,
}

impl TrialFace {
    pub fn new(typed: TypedProgram, definitions: &[BindingId]) -> Self {
        let signatures = definitions
            .iter()
            .map(|binding| typed.signature(*binding))
            .collect();
        Self {
            typed,
            signatures,
            rust: None,
        }
    }

    pub fn typed(&self) -> &TypedProgram {
        &self.typed
    }

    pub fn into_typed(self) -> TypedProgram {
        self.typed
    }

    pub fn changed_signatures(
        &self,
        other: &TrialFace,
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
        let typed = &self.typed;
        self.rust.get_or_insert_with(|| run.emit(typed))
    }
}
