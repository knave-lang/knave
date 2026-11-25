use crate::core::BindingId;

#[derive(Clone, Copy)]
pub enum NewValue {
    Bound(BindingId),
    Replaced(BindingId),
}
