use std::rc::Rc;

use super::enums::{EnumDefinition, VariantDefinition};
use super::pattern_fields::PatternFields;
use super::ty::Type;
use super::unifier::Unifier;
use super::variant_shape::VariantShape;
use crate::core::{Expr, FieldLabel};

pub(super) struct DeclaredVariant<'a> {
    definition: &'a EnumDefinition,
    index: usize,
}

impl<'a> DeclaredVariant<'a> {
    pub(super) fn new(definition: &'a EnumDefinition, index: usize) -> Self {
        Self { definition, index }
    }

    fn variant(&self) -> &VariantDefinition {
        &self.definition.variants[self.index]
    }
}

impl VariantShape for DeclaredVariant<'_> {
    fn storage(&self, arguments: &[Type], _: &mut Unifier) -> Vec<Type> {
        self.definition
            .instantiated_payload(self.index, arguments)
            .unwrap_or_default()
    }

    fn pattern_fields(&self) -> PatternFields<'_> {
        self.variant().pattern_fields()
    }

    fn field_labels(&self, _: &[(FieldLabel, Expr)]) -> Option<Vec<Rc<str>>> {
        self.variant().field_labels()
    }

    fn is_result_ok(&self) -> bool {
        self.definition.is_result_shaped() && self.definition.holds_parameter(self.index, 0)
    }
}
