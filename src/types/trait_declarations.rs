use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::selfless_methods::SelflessMethods;
use super::structural_trait::StructuralTrait;
use super::trait_method_info::TraitMethodInfo;
use super::traits::TraitTable;
use super::typed_trait::TypedTrait;
use super::typed_trait_impl::TypedTraitImpl;
use crate::core::BindingId;

#[derive(Default)]
pub struct TraitDeclarations {
    pub(super) table: TraitTable,
    pub(super) definitions: Vec<TypedTrait>,
    pub(super) impls: Vec<TypedTraitImpl>,
    pub(super) methods: FxHashMap<BindingId, TraitMethodInfo>,
    pub(super) rust_paths: FxHashMap<BindingId, Rc<str>>,
    pub(super) structural: Vec<StructuralTrait>,
    pub(super) selfless_methods: SelflessMethods,
    pub(super) arguments: FxHashSet<BindingId>,
}

impl TraitDeclarations {
    pub fn table(&self) -> &TraitTable {
        &self.table
    }

    pub fn definitions(&self) -> &[TypedTrait] {
        &self.definitions
    }

    pub fn impls(&self) -> &[TypedTraitImpl] {
        &self.impls
    }

    pub fn methods(&self) -> &FxHashMap<BindingId, TraitMethodInfo> {
        &self.methods
    }

    pub fn rust_paths(&self) -> &FxHashMap<BindingId, Rc<str>> {
        &self.rust_paths
    }

    pub fn structural(&self) -> &[StructuralTrait] {
        &self.structural
    }

    pub fn takes_argument(&self, trait_binding: BindingId) -> bool {
        self.arguments.contains(&trait_binding)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "
        (trait listed
          (type item)
          (first (fn ((ref self)) item)))
        (trait drawn (draw (fn ((ref self)) i64)))
        (defn main () (do))";

    #[test]
    fn a_trait_with_an_associated_type_takes_an_argument() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let listed = fixture.trait_named("listed");
        assert!(fixture.traits().takes_argument(listed));
    }

    #[test]
    fn a_trait_without_an_associated_type_takes_no_argument() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let drawn = fixture.trait_named("drawn");
        assert!(!fixture.traits().takes_argument(drawn));
    }
}
