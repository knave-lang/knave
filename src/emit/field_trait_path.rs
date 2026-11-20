use crate::core::{BindingId, ModuleId};
use crate::types::{TypedDefinition, TypedProgram};

pub(super) struct FieldTraitPath<'a> {
    program: &'a TypedProgram,
}

impl<'a> FieldTraitPath<'a> {
    const CRATE_ROOT: &'static str = "crate::";

    pub(super) fn new(program: &'a TypedProgram) -> Self {
        Self { program }
    }

    pub(super) fn in_module(&self, path: &str, module: ModuleId) -> String {
        if module == self.program.entry {
            path.to_string()
        } else {
            format!("{}{path}", Self::CRATE_ROOT)
        }
    }

    pub(super) fn visibility(&self, field_trait: BindingId) -> &'static str {
        let elsewhere =
            (self.program.modules.iter()).filter(|module| module.id != self.program.entry);
        let mut readers = elsewhere.flat_map(|module| &module.definitions);
        if readers.any(|definition| Self::names(definition, field_trait)) {
            "pub "
        } else {
            ""
        }
    }

    fn names(definition: &TypedDefinition, field_trait: BindingId) -> bool {
        let mut bounds = definition
            .scheme
            .params
            .iter()
            .flat_map(|param| &param.bounds);
        bounds.any(|bound| bound.binding == field_trait)
    }
}
