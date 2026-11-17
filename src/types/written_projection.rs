use super::ty::Type;
use crate::core::BindingId;
use crate::source::Span;

pub(super) struct WrittenProjection {
    pub(super) projection: BindingId,
    pub(super) owner: Type,
    pub(super) span: Span,
}
