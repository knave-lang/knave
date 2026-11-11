use std::rc::Rc;

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
