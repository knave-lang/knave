use rustc_hash::FxHashSet;

use super::checking_facts::CheckingFacts;
use super::collected_impl::CollectedImpl;
use super::collected_rust_trait::CollectedRustTrait;
use super::collected_trait::CollectedTrait;
use super::structural_trait::StructuralTrait;
use super::trait_declarations::TraitDeclarations;
use crate::core::BindingId;

pub(super) struct TraitDeclarationsBuilder<'a> {
    traits: &'a mut TraitDeclarations,
}

impl<'a> TraitDeclarationsBuilder<'a> {
    pub(super) fn new(traits: &'a mut TraitDeclarations) -> Self {
        Self { traits }
    }

    pub(super) fn enter_trait(
        self,
        collected: CollectedTrait,
        facts: &mut CheckingFacts,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        collected.enter(self.traits, facts, synthetic);
    }

    pub(super) fn enter_rust_trait(
        self,
        collected: CollectedRustTrait,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        collected.enter(self.traits, synthetic);
    }

    pub(super) fn enter_impl(
        self,
        collected: CollectedImpl,
        synthetic: &mut FxHashSet<BindingId>,
        impl_methods: &mut FxHashSet<BindingId>,
    ) {
        collected.enter(self.traits, synthetic, impl_methods);
    }

    pub(super) fn add_structural(self, structural: StructuralTrait) {
        self.traits.structural.push(structural);
    }
}
