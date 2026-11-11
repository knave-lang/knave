use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::selfless_methods::SelflessMethods;
use super::structural_trait::StructuralTrait;
use super::traits::TraitTable;
use super::typed::{TraitMethodInfo, TypedTrait, TypedTraitImpl};
use crate::core::BindingId;

#[derive(Default)]
pub(super) struct TraitDeclarations {
    pub(super) table: TraitTable,
    pub(super) definitions: Vec<TypedTrait>,
    pub(super) impls: Vec<TypedTraitImpl>,
    pub(super) methods: FxHashMap<BindingId, TraitMethodInfo>,
    pub(super) rust_paths: FxHashMap<BindingId, Rc<str>>,
    pub(super) structural: Vec<StructuralTrait>,
    pub(super) selfless_methods: SelflessMethods,
}
