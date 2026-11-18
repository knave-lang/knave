use rustc_hash::FxHashSet;

use super::trait_declarations::TraitDeclarations;
use super::{Implementor, TypedTraitImpl};
use crate::core::BindingId;

pub(super) struct CollectedImpl {
    implementor: Implementor,
    definition: TypedTraitImpl,
}

impl CollectedImpl {
    pub(super) fn new(implementor: Implementor, definition: TypedTraitImpl) -> Self {
        Self {
            implementor,
            definition,
        }
    }

    pub(super) fn enter(
        self,
        traits: &mut TraitDeclarations,
        synthetic: &mut FxHashSet<BindingId>,
        impl_methods: &mut FxHashSet<BindingId>,
    ) {
        traits
            .table
            .add_implementor(self.definition.trait_binding, self.implementor);
        for method in &self.definition.methods {
            synthetic.insert(method.binding);
            impl_methods.insert(method.binding);
        }
        traits.impls.push(self.definition);
    }
}
