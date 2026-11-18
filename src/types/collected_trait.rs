use super::selfless_method::SelflessMethod;
use super::{RustTrait, TypedTrait};
use crate::core::BindingId;

pub(super) struct CollectedTrait {
    pub(super) rust_trait: RustTrait,
    pub(super) definition: TypedTrait,
    pub(super) selfless: Vec<(BindingId, SelflessMethod)>,
}

impl CollectedTrait {
    pub(super) fn new(
        rust_trait: RustTrait,
        definition: TypedTrait,
        selfless: Vec<(BindingId, SelflessMethod)>,
    ) -> Self {
        Self {
            rust_trait,
            definition,
            selfless,
        }
    }
}
