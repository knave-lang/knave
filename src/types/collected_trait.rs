use rustc_hash::FxHashSet;

use super::checking_facts::CheckingFacts;
use super::selfless_method::SelflessMethod;
use super::trait_declarations::TraitDeclarations;
use super::{RustTrait, TypedTrait};
use crate::core::BindingId;

pub(super) struct CollectedTrait {
    rust_trait: RustTrait,
    definition: TypedTrait,
    selfless: Vec<(BindingId, SelflessMethod)>,
}

impl CollectedTrait {
    pub(super) fn new(
        rust_trait: RustTrait,
        definition: TypedTrait,
        selfless: Vec<(BindingId, SelflessMethod)>,
    ) -> Self {
        Self {
            rust_trait,
            definition,
            selfless,
        }
    }

    pub(super) fn enter(
        self,
        traits: &mut TraitDeclarations,
        facts: &mut CheckingFacts,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        traits.table.add(self.definition.binding, self.rust_trait);
        for (method, selfless) in self.selfless {
            facts.selfless_methods.insert(method, selfless);
        }
        for method in &self.definition.methods {
            synthetic.extend(std::iter::once(method.binding).chain(method.default));
            traits.enter_method(method.binding, self.definition.method_info(method));
        }
        traits.definitions.push(self.definition);
    }
}
