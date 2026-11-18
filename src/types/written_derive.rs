use crate::core::BindingId;
use crate::source::Span;

pub(super) struct WrittenDerive {
    pub(super) span: Span,
    pub(super) traits: Vec<(BindingId, Span)>,
}
