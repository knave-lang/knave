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
        match ty {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.structure_of(inner),
            Type::Named(binding, _) => self
                .program
                .declarations
                .types()
                .structs()
                .get(*binding)
                .map(|structure| (*binding, structure)),
            _ => None,
        }
    }

    pub fn payload_argument(&self, variant: BindingId, field: usize) -> Option<usize> {
        let BindingKind::Variant(_, info) = self.program.bindings.get(variant).kind() else {
            return None;
        };
        let definition = self.program.declarations.types().enums().get(info.owner)?;
        let payload = match &definition.variants.get(info.index)?.payload {
            VariantPayload::Positional(types) => types.get(field)?,
            VariantPayload::Named(fields) => &fields.get(field)?.1,
            VariantPayload::None => return None,
        };
        match payload {
            Type::Generic(generic) => definition
                .params
                .iter()
                .position(|param| param.id == *generic),
            _ => None,
        }
    }

    pub fn collection_of(&self, ty: &Type) -> Option<Collection> {
        Collection::of(ty, self.program.declarations.types().rust_types())
    }
}
