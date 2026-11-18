use crate::core::{BindingId, EnumDefiner, EnumRust, TypeBinder, VariantDeclaration};

#[derive(Clone, Copy)]
pub(super) struct EnumItem<'a> {
    pub(super) binding: BindingId,
    pub(super) binders: &'a [TypeBinder],
    pub(super) variants: &'a [VariantDeclaration],
    pub(super) rust: &'a Option<EnumRust>,
    pub(super) definer: &'a Option<EnumDefiner>,
}
