use crate::core::BindingId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Bound(BindingId),
    Unbound,
    Ambiguous(Vec<BindingId>),
}
