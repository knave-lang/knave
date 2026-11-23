use crate::types::ty::Type;

pub(in crate::types) trait Obligations {
    fn require_copy(&mut self, value: &Type);
}
