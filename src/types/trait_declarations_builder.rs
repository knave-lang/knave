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
        let traits = self.traits;
        traits
            .table
            .add(collected.definition.binding, collected.rust_trait);
        for (method, selfless) in collected.selfless {
            facts.selfless_methods.insert(method, selfless);
        }
        for method in &collected.definition.methods {
            synthetic.extend(std::iter::once(method.binding).chain(method.default));
            let info = collected.definition.method_info(method);
            traits.methods.insert(method.binding, info);
        }
        traits.definitions.push(collected.definition);
    }

    pub(super) fn enter_rust_trait(
        self,
        collected: CollectedRustTrait,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        let traits = self.traits;
        traits
            .table
            .add(collected.declaration.binding, collected.rust_trait);
        traits.rust_paths.record(collected.declaration);
        for (method, info) in collected.methods {
            synthetic.insert(method);
            traits.methods.insert(method, info);
        }
    }

    pub(super) fn enter_impl(
        self,
        collected: CollectedImpl,
        synthetic: &mut FxHashSet<BindingId>,
        impl_methods: &mut FxHashSet<BindingId>,
    ) {
        let traits = self.traits;
        traits
            .table
            .add_implementor(collected.definition.trait_binding, collected.implementor);
        for method in &collected.definition.methods {
            synthetic.insert(method.binding);
            impl_methods.insert(method.binding);
        }
        traits.impls.push(collected.definition);
    }

    pub(super) fn add_structural(self, structural: StructuralTrait) {
        self.traits.structural.push(structural);
    }
}
