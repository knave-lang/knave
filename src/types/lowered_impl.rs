use super::ty::{GenericParam, Type};
use crate::core::BindingId;

pub(super) struct LoweredImpl {
    implementor: Type,
    params: Vec<GenericParam>,
    associated: Vec<(BindingId, Type)>,
}

impl LoweredImpl {
    pub(super) fn new(
        implementor: Type,
        params: Vec<GenericParam>,
        associated: Vec<(BindingId, Type)>,
    ) -> Self {
        Self {
            implementor,
            params,
            associated,
        }
    }

    pub(super) fn implementor(&self) -> &Type {
        &self.implementor
    }

    pub(super) fn into_parts(self) -> (Type, Vec<GenericParam>, Vec<(BindingId, Type)>) {
        (self.implementor, self.params, self.associated)
    }
}
