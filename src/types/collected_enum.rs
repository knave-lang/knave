use super::RustType;
use super::enums::EnumDefinition;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) struct CollectedEnum {
    pub(super) definition: EnumDefinition,
    pub(super) rust_type: Option<RustType>,
    pub(super) problems: Vec<Diagnostic>,
}

impl CollectedEnum {
    pub(super) fn enter(
        self,
        binding: BindingId,
        types: &mut TypeDeclarations,
        diagnostics: &mut Diagnostics,
    ) {
        if let Some(rust_type) = self.rust_type {
            types.rust_types.add(binding, rust_type);
        }
        types.enums.add(binding, self.definition);
        diagnostics.extend(self.problems.into_iter().collect());
    }
}
