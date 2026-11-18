use super::RustType;
use super::enums::EnumDefinition;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) struct CollectedEnum {
    definition: EnumDefinition,
    rust_type: Option<RustType>,
    problems: Vec<Diagnostic>,
}

impl CollectedEnum {
    pub(super) fn new(
        definition: EnumDefinition,
        rust_type: Option<RustType>,
        problems: Vec<Diagnostic>,
    ) -> Self {
        Self {
            definition,
            rust_type,
            problems,
        }
    }

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
