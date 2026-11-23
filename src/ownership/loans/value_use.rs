use crate::core::BindingId;
use crate::source::Span;

pub struct ValueUse {
    pub binding: BindingId,
    pub span: Span,
}
