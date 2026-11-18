use super::trait_declarations::TraitDeclarations;
use super::{ReceiverKind, TypedTrait};
use crate::core::BindingId;

pub(super) struct TraitDefinitions<'a> {
    traits: &'a TraitDeclarations,
}

impl<'a> TraitDefinitions<'a> {
    pub(super) fn new(traits: &'a TraitDeclarations) -> Self {
        Self { traits }
    }

    pub(super) fn definition(&self, trait_binding: BindingId) -> Option<&'a TypedTrait> {
        self.traits
            .definitions
            .iter()
            .find(|definition| definition.binding == trait_binding)
    }

    pub(super) fn receiver(&self, trait_method: BindingId) -> Option<ReceiverKind> {
        self.traits
            .methods
            .get(&trait_method)
            .and_then(|info| info.receiver)
    }
}
