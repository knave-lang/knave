use std::rc::Rc;

use super::ty::{GenericParam, Type};
use super::typed_impl_method::TypedImplMethod;
use crate::core::{BindingId, ModuleId};
use crate::source::Span;

pub struct TypedTraitImpl {
    pub trait_binding: BindingId,
    pub implementor: Type,
    pub params: Vec<GenericParam>,
    pub associated: Vec<(BindingId, Rc<str>, Type)>,
    pub implementor_span: Span,
    pub module: ModuleId,
    pub methods: Vec<TypedImplMethod>,
}
