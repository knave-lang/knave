use crate::types::ty::Type;

pub(in crate::types) trait Unknowns {
    fn fresh(&mut self) -> Type;
    fn resolve(&self, value: &Type) -> Type;
    fn is_open(&self, value: &Type) -> bool;
}
