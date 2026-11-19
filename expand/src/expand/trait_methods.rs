use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::core::{BindingId, TypeExpr};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TraitMethods {
    pub rust_name: Rc<str>,
    pub self_variable: BindingId,
    pub associated: Vec<(BindingId, Rc<str>)>,
    pub methods: Vec<(BindingId, TypeExpr)>,
    pub defaults: FxHashSet<BindingId>,
}
