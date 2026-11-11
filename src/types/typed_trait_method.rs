use super::receiver_kind::ReceiverKind;
use crate::core::BindingId;

#[derive(Clone)]
pub struct TypedTraitMethod {
    pub binding: BindingId,
    pub receiver: Option<ReceiverKind>,
    pub default: Option<BindingId>,
    pub returns_self: bool,
}

impl TypedTraitMethod {
    pub fn declaration(&self) -> BindingId {
        self.default.unwrap_or(self.binding)
    }
}
