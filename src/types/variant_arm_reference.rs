use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct VariantArmReference {
    pub(super) outer: Option<BindingId>,
    pub(super) variant: BindingId,
}
