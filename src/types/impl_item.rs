use crate::core::{BindingId, ImplAssociatedType, ImplMethod, ModuleId, TypeExpr};

#[derive(Clone, Copy)]
pub(super) struct ImplItem<'a> {
    pub(super) module: ModuleId,
    pub(super) trait_binding: BindingId,
    pub(super) implementor: &'a TypeExpr,
    pub(super) associated: &'a [ImplAssociatedType],
    pub(super) methods: &'a [ImplMethod],
}
