use crate::core::BindingId;
use crate::types::ty::Type;

pub(in crate::types) trait LocalTypes {
    fn local_type(&self, binding: BindingId) -> Type;
}
