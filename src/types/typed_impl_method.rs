use super::receiver_kind::ReceiverKind;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub struct TypedImplMethod {
    pub binding: BindingId,
    pub receiver: Option<ReceiverKind>,
}
