use crate::core::{BindingId, BindingKind};

pub(in crate::types) trait Bindings {
    fn binding_name(&self, binding: BindingId) -> String;
    fn binding_kind(&self, binding: BindingId) -> BindingKind;
}
