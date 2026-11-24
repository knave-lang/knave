use crate::core::BindingId;
use crate::types::variant_arm_reference::VariantArmReference;

pub(in crate::types) trait VariantArms {
    fn variant_arm(&self, binding: BindingId) -> Option<VariantArmReference>;
}
