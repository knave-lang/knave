use super::enum_payloads::EnumPayloads;
use crate::core::BindingId;
use crate::types::{Collection, StructDefinition, Type, TypedProgram};

#[derive(Clone, Copy)]
pub struct TextTypes<'p> {
    program: &'p TypedProgram,
}

impl<'p> TextTypes<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn structure_of(&self, ty: &Type) -> Option<(BindingId, &'p StructDefinition)> {
        self.program
            .declarations
            .types()
            .structs()
            .behind(ty)
            .map(|found| (found.binding, found.definition))
    }

    pub fn payloads(&self) -> EnumPayloads<'p> {
        EnumPayloads::new(self.program)
    }

    pub fn collection_of(&self, ty: &Type) -> Option<Collection> {
        Collection::of(ty, self.program.declarations.types().rust_types())
    }
}
