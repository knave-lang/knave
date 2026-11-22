use std::rc::Rc;

use super::slot_owner::SlotOwner;
use crate::core::{BindingId, BindingKind, VariantInfo};
use crate::types::{EnumDefinition, Type, TypedProgram, VariantDefinition, VariantPayload};

#[derive(Clone, Copy)]
pub struct EnumPayloads<'p> {
    program: &'p TypedProgram,
}

impl<'p> EnumPayloads<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn argument(&self, variant: BindingId, field: usize) -> Option<usize> {
        let (definition, case) = self.case(variant)?;
        let Type::Generic(generic) = Self::field_type(case, field)? else {
            return None;
        };
        definition
            .params
            .iter()
            .position(|param| param.id == *generic)
    }

    pub fn field(&self, variant: BindingId, field: usize) -> Option<(SlotOwner, &'p Type)> {
        let info = self.info(variant)?;
        let (_, case) = self.case(variant)?;
        let owner = SlotOwner::Field(info.owner, Self::label(case, field));
        Some((owner, Self::field_type(case, field)?))
    }

    pub fn enumeration(&self, binding: BindingId) -> Option<&'p EnumDefinition> {
        self.program.declarations.types().enums().get(binding)
    }

    pub fn label(case: &VariantDefinition, field: usize) -> Rc<str> {
        match &case.payload {
            VariantPayload::Named(fields) => format!("{}.{}", case.name, fields[field].0).into(),
            _ => format!("{}.{field}", case.name).into(),
        }
    }

    fn case(&self, variant: BindingId) -> Option<(&'p EnumDefinition, &'p VariantDefinition)> {
        let info = self.info(variant)?;
        let definition = self.enumeration(info.owner)?;
        Some((definition, definition.variants.get(info.index)?))
    }

    fn info(&self, variant: BindingId) -> Option<VariantInfo> {
        match self.program.bindings.get(variant).kind() {
            BindingKind::Variant(_, info) => Some(info),
            _ => None,
        }
    }

    fn field_type(case: &'p VariantDefinition, field: usize) -> Option<&'p Type> {
        match &case.payload {
            VariantPayload::Positional(types) => types.get(field),
            VariantPayload::Named(fields) => fields.get(field).map(|(_, ty)| ty),
            VariantPayload::None => None,
        }
    }
}
