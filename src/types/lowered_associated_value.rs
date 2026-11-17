use super::ty::Type;
use crate::core::BindingId;
use crate::source::Span;

pub(super) struct LoweredAssociatedValue {
    pub projection: BindingId,
    pub value: Type,
    pub span: Span,
}
