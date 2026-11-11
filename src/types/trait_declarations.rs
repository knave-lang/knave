use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::selfless_methods::SelflessMethods;
use super::structural_trait::StructuralTrait;
use super::trait_method_info::TraitMethodInfo;
use super::traits::TraitTable;
use super::typed_trait::TypedTrait;
use super::typed_trait_impl::TypedTraitImpl;
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

#[cfg(test)]
impl TraitDeclarations {
    pub(super) fn taken_from(program: &mut super::TypedProgram) -> Self {
        Self {
            table: std::mem::take(&mut program.traits),
            impls: std::mem::take(&mut program.trait_impls),
            structural: std::mem::take(&mut program.structural_traits),
            ..Self::default()
        }
    }
}
