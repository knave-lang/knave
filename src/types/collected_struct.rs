use super::StructDefinition;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) struct CollectedStruct {
    definition: StructDefinition,
    problem: Option<Diagnostic>,
}

impl CollectedStruct {
    pub(super) fn new(definition: StructDefinition, problem: Option<Diagnostic>) -> Self {
        Self {
            definition,
            problem,
        }
    }

    pub(super) fn enter(
        self,
        binding: BindingId,
        types: &mut TypeDeclarations,
        diagnostics: &mut Diagnostics,
    ) {
        types.structs.add(binding, self.definition);
        diagnostics.extend(self.problem.into_iter().collect());
    }
}
