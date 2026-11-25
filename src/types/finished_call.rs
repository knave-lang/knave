use super::ty::Type;
use super::typed::TypedArgument;
use crate::source::Span;

pub(super) struct FinishedCall<'a> {
    pub(super) template: &'a str,
    pub(super) parameters: Vec<Type>,
    pub(super) result: Type,
    pub(super) arguments: Vec<TypedArgument>,
    pub(super) span: Span,
}
