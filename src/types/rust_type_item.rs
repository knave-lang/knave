use std::rc::Rc;

use crate::core::{ParsedTemplate, TypeBinder, TypeExpr};

pub(super) struct RustTypeItem<'a> {
    pub(super) binders: &'a [TypeBinder],
    pub(super) unsized_parameters: &'a [usize],
    pub(super) template: &'a Rc<ParsedTemplate>,
    pub(super) clauses: &'a [(TypeExpr, Rc<ParsedTemplate>)],
}
