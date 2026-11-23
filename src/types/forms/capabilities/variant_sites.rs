use crate::core::{BindingId, VariantInfo};
use crate::types::ty::Type;
use crate::types::variant_site::VariantSite;

pub(in crate::types) trait VariantSites {
    fn variant_site(&mut self, binding: BindingId, expected: Option<&Type>) -> VariantSite;
    fn is_result_ok(&self, info: VariantInfo) -> bool;
}
