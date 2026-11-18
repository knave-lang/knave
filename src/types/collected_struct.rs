use super::StructDefinition;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) struct CollectedStruct {
    pub(super) definition: StructDefinition,
    pub(super) problem: Option<Diagnostic>,
}

impl CollectedStruct {
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
