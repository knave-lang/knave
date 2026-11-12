use crate::core::{BindingId, VariantInfo};
use crate::types::Type;

pub(super) struct VariantTarget {
    pub(super) binding: BindingId,
    pub(super) info: VariantInfo,
    pub(super) arguments: Vec<Type>,
}

impl VariantTarget {
    pub(super) fn new(binding: BindingId, info: VariantInfo, arguments: Vec<Type>) -> Self {
        Self {
            binding,
            info,
            arguments,
        }
    }
}
