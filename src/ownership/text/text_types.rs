use crate::core::{BindingId, BindingKind};
use crate::types::{Collection, StructDefinition, Type, TypedProgram, VariantPayload};

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

    pub fn payload_argument(&self, variant: BindingId, field: usize) -> Option<usize> {
        let BindingKind::Variant(_, info) = self.program.bindings.get(variant).kind() else {
            return None;
        };
        let definition = self.program.declarations.types().enums().get(info.owner)?;
        let payload = Self::payload_type(&definition.variants.get(info.index)?.payload, field)?;
        let Type::Generic(generic) = payload else {
            return None;
        };
        definition
            .params
            .iter()
            .position(|param| param.id == *generic)
    }

    fn payload_type(payload: &VariantPayload, field: usize) -> Option<&Type> {
        match payload {
            VariantPayload::Positional(types) => types.get(field),
            VariantPayload::Named(fields) => fields.get(field).map(|(_, ty)| ty),
            VariantPayload::None => None,
        }
    }

    pub fn collection_of(&self, ty: &Type) -> Option<Collection> {
        Collection::of(ty, self.program.declarations.types().rust_types())
    }
}
