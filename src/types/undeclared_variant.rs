use std::rc::Rc;

use super::pattern_fields::PatternFields;
use super::ty::Type;
use super::unifier::Unifier;
use super::variant_shape::VariantShape;
use crate::core::{Expr, FieldLabel};

pub(super) struct UndeclaredVariant {
    arity: usize,
}

impl UndeclaredVariant {
    pub(super) fn new(arity: usize) -> Self {
        Self { arity }
    }
}

impl VariantShape for UndeclaredVariant {
    fn storage(&self, _: &[Type], unifier: &mut Unifier) -> Vec<Type> {
        (0..self.arity).map(|_| unifier.fresh()).collect()
    }

    fn pattern_fields(&self) -> PatternFields<'_> {
        PatternFields::Positional(self.arity)
    }

    fn field_labels(&self, given: &[(FieldLabel, Expr)]) -> Option<Vec<Rc<str>>> {
        Some(given.iter().map(|(label, _)| label.name.clone()).collect())
    }

    fn is_result_ok(&self) -> bool {
        false
    }
}
