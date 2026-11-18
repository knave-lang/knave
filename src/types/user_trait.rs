use std::rc::Rc;

use crate::core::{BindingId, ModuleId, TraitAssociatedType, TraitMethod, TypeBound};

#[derive(Clone, Copy)]
pub(super) struct UserTrait<'a> {
    pub(super) module: ModuleId,
    pub(super) binding: BindingId,
    pub(super) rust_name: &'a Rc<str>,
    pub(super) self_variable: BindingId,
    pub(super) supertraits: &'a [TypeBound],
    pub(super) associated: &'a [TraitAssociatedType],
    pub(super) methods: &'a [TraitMethod],
}

impl UserTrait<'_> {
    pub(super) fn takes_argument(&self) -> bool {
        !self.associated.is_empty()
    }
}
