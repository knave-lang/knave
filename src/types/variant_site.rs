use super::ty::Type;
use crate::core::VariantInfo;

pub(super) struct VariantSite {
    pub(super) info: VariantInfo,
    pub(super) type_arguments: Vec<Type>,
    pub(super) payload: Vec<Type>,
}
