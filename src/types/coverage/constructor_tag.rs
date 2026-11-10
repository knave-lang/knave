use crate::core::BindingId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConstructorTag {
    Variant { owner: BindingId, index: usize },
    Struct(BindingId),
    Bool(bool),
}
