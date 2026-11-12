use std::rc::Rc;

use super::pattern_fields::PatternFields;
use super::ty::Type;
use super::unifier::Unifier;
use crate::core::{Expr, FieldLabel};

pub(super) trait VariantShape {
    fn storage(&self, arguments: &[Type], unifier: &mut Unifier) -> Vec<Type>;

    fn pattern_fields(&self) -> PatternFields<'_>;

    fn field_labels(&self, given: &[(FieldLabel, Expr)]) -> Option<Vec<Rc<str>>>;

    fn is_result_ok(&self) -> bool;
}
