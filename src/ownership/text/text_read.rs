use super::transfer::Transfer;
use crate::source::Span;
use crate::types::{Type, TypedExpr};

pub struct TextRead<'t> {
    pub ty: &'t Type,
    pub span: Span,
    pub transfer: Transfer,
}

impl<'t> TextRead<'t> {
    pub fn of(expr: &'t TypedExpr, transfer: Transfer) -> Self {
        Self {
            ty: &expr.ty,
            span: expr.span,
            transfer,
        }
    }
}
