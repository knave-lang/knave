use super::payload_position::PayloadPosition;
use crate::core::BindingId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoxedPayload {
    pub owner: BindingId,
    pub position: PayloadPosition,
}
