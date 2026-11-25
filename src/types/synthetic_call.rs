use super::synthetic_signature::SyntheticSignature;
use super::typed::TypedExpr;
use crate::core::Expr;
use crate::source::Span;

pub(super) struct SyntheticCall<'a> {
    pub(super) signature: SyntheticSignature<'a>,
    pub(super) value: TypedExpr,
    pub(super) arguments: &'a [Expr],
    pub(super) span: Span,
}
