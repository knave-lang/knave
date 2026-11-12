use std::rc::Rc;

use super::enums::VariantDefinition;
use super::pattern_fields::PatternFields;
use super::ty::Type;
use super::undeclared_variant::UndeclaredVariant;
use super::unifier::Unifier;
use super::variant_shape::VariantShape;
use crate::core::{Expr, FieldLabel};

pub(super) struct UnloweredVariant<'a> {
    variant: &'a VariantDefinition,
    unknown: UndeclaredVariant,
}

impl<'a> UnloweredVariant<'a> {
    pub(super) fn new(variant: &'a VariantDefinition) -> Self {
        Self {
            variant,
            unknown: UndeclaredVariant::new(variant.arity()),
        }
    }
}

impl VariantShape for UnloweredVariant<'_> {
    fn storage(&self, arguments: &[Type], unifier: &mut Unifier) -> Vec<Type> {
        self.unknown.storage(arguments, unifier)
    }

    fn pattern_fields(&self) -> PatternFields<'_> {
        self.variant.pattern_fields()
    }

    fn field_labels(&self, _: &[(FieldLabel, Expr)]) -> Option<Vec<Rc<str>>> {
        self.variant.field_labels()
    }

    fn is_result_ok(&self) -> bool {
        self.unknown.is_result_ok()
    }
}
