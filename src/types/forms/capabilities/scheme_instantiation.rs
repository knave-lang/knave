use crate::types::ty::{Scheme, Type};

pub(in crate::types) trait SchemeInstantiation {
    fn instantiate(&mut self, scheme: &Scheme) -> Type;
}
