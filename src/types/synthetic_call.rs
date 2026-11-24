use super::ty::Type;
use super::typed::TypedExpr;
use crate::core::Expr;
use crate::source::Span;

pub(super) struct SyntheticCall<'a> {
    pub(super) template: &'a str,
    pub(super) parameters: Vec<Type>,
    pub(super) result: Type,
    pub(super) value: TypedExpr,
    pub(super) arguments: &'a [Expr],
    pub(super) span: Span,
}
