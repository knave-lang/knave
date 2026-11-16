use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone, Debug, Default)]
pub struct ClosureFacts {
    pub moved_capture: Option<(BindingId, Span)>,
    pub changed_capture: Option<(BindingId, Span)>,
}
