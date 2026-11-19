use crate::core::BindingId;

pub(super) struct ViewedArguments {
    pub(super) view: BindingId,
    pub(super) positions: Vec<usize>,
}
