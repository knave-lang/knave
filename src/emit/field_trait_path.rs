use crate::core::{BindingId, ModuleId};
use crate::types::{FieldGetterName, FieldTraits, TypedDefinition, TypedProgram};

pub(super) struct FieldTraitPath<'a> {
    program: &'a TypedProgram,
}

impl<'a> FieldTraitPath<'a> {
    const CRATE_ROOT: &'static str = "crate::";

    pub(super) fn new(program: &'a TypedProgram) -> Self {
        Self { program }
    }

    pub(super) fn of(&self, field_trait: BindingId, from: ModuleId) -> Option<String> {
        let name = &self
            .program
            .declarations
            .traits()
            .table()
            .get(field_trait)?
            .path;
        let root = if from == self.program.entry {
            ""
        } else {
            Self::CRATE_ROOT
        };
        Some(format!("{root}{}::{name}", FieldTraits::MODULE))
    }

    pub(super) fn getter(&self, field: &str, from: ModuleId) -> Option<String> {
        let field_traits = self
            .program
            .declarations
            .traits()
            .rust_paths()
            .field_traits();
        let path = self.of(field_traits.bindings().trait_of(field)?, from)?;
        Some(format!("{path}::{}", FieldGetterName::of(field)))
    }

    pub(super) fn visibility(&self) -> &'static str {
        let elsewhere =
            (self.program.modules.iter()).filter(|module| module.id != self.program.entry);
        let mut readers = elsewhere.flat_map(|module| &module.definitions);
        if readers.any(|definition| self.names_field_trait(definition)) {
            "pub "
        } else {
            ""
        }
    }

    fn names_field_trait(&self, definition: &TypedDefinition) -> bool {
        let field_traits = self
            .program
            .declarations
            .traits()
            .rust_paths()
            .field_traits();
        let mut bounds = (definition.scheme.params.iter()).flat_map(|param| &param.bounds);
        bounds.any(|bound| field_traits.bindings().field_of(bound.binding).is_some())
    }
}
