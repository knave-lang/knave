use super::enums::{EnumDefinition, VariantDefinition};
use super::ty::Type;
use super::typed::TypedProgram;
use crate::core::{BindingId, BindingKind, VariantInfo};

#[derive(Clone, Copy)]
pub struct EnumCases<'p> {
    program: &'p TypedProgram,
}

impl<'p> EnumCases<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn enumeration(&self, binding: BindingId) -> Option<&'p EnumDefinition> {
        self.program.declarations.types().enums().get(binding)
    }

    pub fn owner_of(&self, variant: BindingId) -> Option<BindingId> {
        self.info(variant).map(|info| info.owner)
    }

    pub fn case(&self, variant: BindingId) -> Option<(&'p EnumDefinition, &'p VariantDefinition)> {
        let info = self.info(variant)?;
        let definition = self.enumeration(info.owner)?;
        Some((definition, definition.variants.get(info.index)?))
    }

    pub fn type_parameter_of(&self, variant: BindingId, field: usize) -> Option<usize> {
        let (definition, case) = self.case(variant)?;
        let Type::Generic(generic) = case.field_type(field)? else {
            return None;
        };
        definition
            .params
            .iter()
            .position(|param| param.id == *generic)
    }

    fn info(&self, variant: BindingId) -> Option<VariantInfo> {
        match self.program.bindings.get(variant).kind() {
            BindingKind::Variant(_, info) => Some(info),
            _ => None,
        }
    }
}
