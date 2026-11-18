use super::derived_trait::DerivedTrait;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct DerivableTrait {
    pub(super) binding: BindingId,
    pub(super) derived: DerivedTrait,
}
