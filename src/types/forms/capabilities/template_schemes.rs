use std::rc::Rc;

use crate::core::TypeExpr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::ty::{Scheme, Type};

pub(in crate::types) trait TemplateSchemes {
    fn lower_template(&mut self, type_expr: &TypeExpr) -> Result<Scheme, Diagnostic>;
    fn register_inline(&mut self, text: &Rc<str>, scheme: &Scheme);
    fn instantiate(&mut self, scheme: &Scheme, span: Span) -> Type;
}
