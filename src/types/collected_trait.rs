use rustc_hash::FxHashSet;

use super::checking_facts::CheckingFacts;
use super::trait_declarations::TraitDeclarations;
use super::user_trait::UserTrait;
use super::{RustTrait, TraitMethodInfo, TypedTrait};
use crate::core::BindingId;

pub(super) struct CollectedTrait<'a> {
    declaration: UserTrait<'a>,
    rust_trait: RustTrait,
    definition: TypedTrait,
}

impl<'a> CollectedTrait<'a> {
    pub(super) fn new(
        declaration: UserTrait<'a>,
        rust_trait: RustTrait,
        definition: TypedTrait,
    ) -> Self {
        Self {
            declaration,
            rust_trait,
            definition,
        }
    }

    pub(super) fn enter(
        self,
        traits: &mut TraitDeclarations,
        facts: &mut CheckingFacts,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        let binding = self.declaration.binding;
        traits.table.add(binding, self.rust_trait);
        for (method, typed) in self
            .declaration
            .methods
            .iter()
            .zip(&self.definition.methods)
        {
            facts
                .selfless_methods
                .record(method, binding, self.declaration.self_variable);
            synthetic.insert(method.binding);
            if let Some(default) = method.default {
                synthetic.insert(default);
            }
            traits.methods.insert(
                method.binding,
                TraitMethodInfo {
                    trait_binding: binding,
                    rust_name: self.declaration.rust_name.clone(),
                    receiver: typed.receiver,
                },
            );
        }
        traits.definitions.push(self.definition);
    }
}
