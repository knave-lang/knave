use std::rc::Rc;

use super::slot_owner::SlotOwner;
use crate::core::BindingId;
use crate::types::{
    EnumCases, EnumDefinition, Type, TypedProgram, VariantDefinition, VariantPayload,
};

#[derive(Clone, Copy)]
pub struct EnumPayloads<'p> {
    program: &'p TypedProgram,
    cases: EnumCases<'p>,
}

impl<'p> EnumPayloads<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            cases: EnumCases::new(program),
        }
    }

    pub fn argument(&self, variant: BindingId, field: usize) -> Option<usize> {
        self.cases.type_parameter_of(variant, field)
    }

    pub fn field(&self, variant: BindingId, field: usize) -> Option<(SlotOwner, &'p Type)> {
        let (_, case) = self.cases.case(variant)?;
        let owner = SlotOwner::Field(self.cases.owner_of(variant)?, Self::label(case, field));
        Some((owner, case.payload_types().get(field).copied()?))
    }

    pub fn converted_case(&self, target: BindingId, error: &Type) -> Option<SlotOwner> {
        let conversion =
            self.program.conversions.iter().find(|conversion| {
                conversion.enum_binding == target && conversion.error == *error
            })?;
        let case = self.enumeration(target)?.variants.get(conversion.variant)?;
        Some(SlotOwner::Field(target, Self::label(case, 0)))
    }

    pub fn enumeration(&self, binding: BindingId) -> Option<&'p EnumDefinition> {
        self.cases.enumeration(binding)
    }

    pub fn label(case: &VariantDefinition, field: usize) -> Rc<str> {
        match &case.payload {
            VariantPayload::Named(fields) => format!("{}.{}", case.name, fields[field].0).into(),
            _ => format!("{}.{field}", case.name).into(),
        }
    }
}
