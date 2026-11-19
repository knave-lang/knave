use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::declarations::Declarations;
use super::ty::{GenericId, GenericParam};
use crate::core::{BindingArena, BindingId};

#[derive(Clone, Copy)]
pub(super) struct PassingSources<'a> {
    pub(super) declarations: &'a Declarations,
    pub(super) bindings: &'a BindingArena,
    pub(super) generics: &'a FxHashMap<GenericId, GenericParam>,
    pub(super) single_field_readers: &'a FxHashMap<BindingId, Rc<str>>,
    pub(super) copy_trait: Option<BindingId>,
}
