use crate::core::BindingId;
use crate::types::ty::Type;

pub(in crate::types) trait LocalScope {
    fn bind_local(&mut self, binding: BindingId, ty: Type);
}
