use std::rc::Rc;

use super::receiver_kind::ReceiverKind;
use crate::core::BindingId;

pub struct TraitMethodInfo {
    pub trait_binding: BindingId,
    pub rust_name: Rc<str>,
    pub receiver: Option<ReceiverKind>,
}
