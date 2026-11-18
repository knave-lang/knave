use std::rc::Rc;

use super::trait_method_info::TraitMethodInfo;
use super::ty::Bound;
use super::typed_associated_type::TypedAssociatedType;
use super::typed_trait_method::TypedTraitMethod;
use crate::core::{BindingId, ModuleId};

pub struct TypedTrait {
    pub binding: BindingId,
    pub module: ModuleId,
    pub rust_name: Rc<str>,
    pub supertraits: Vec<Bound>,
    pub associated: Vec<TypedAssociatedType>,
    pub methods: Vec<TypedTraitMethod>,
}

impl TypedTrait {
    pub(super) fn method_info(&self, method: &TypedTraitMethod) -> TraitMethodInfo {
        TraitMethodInfo {
            trait_binding: self.binding,
            rust_name: self.rust_name.clone(),
            receiver: method.receiver,
        }
    }
}
