use super::selection::Selection;
use super::trait_declarations::TraitDeclarations;
use super::ty::{Bound, Type};
use super::typed::TypedTraitImpl;
use crate::core::{BindingId, FunctionFieldTraits, StructDerives};

#[derive(Clone, Copy)]
pub(super) struct DeclaredTraits<'a> {
    traits: &'a TraitDeclarations,
}

impl<'a> DeclaredTraits<'a> {
    pub(super) fn new(traits: &'a TraitDeclarations) -> Self {
        Self { traits }
    }

    pub(super) fn is_derivable(&self, trait_binding: BindingId) -> bool {
        self.derive_name_is_one_of(trait_binding, &StructDerives::DERIVABLE)
            || self.is_structural(trait_binding)
    }

    pub(super) fn is_structural(&self, trait_binding: BindingId) -> bool {
        self.traits
            .structural
            .iter()
            .any(|structural| structural.binding == trait_binding)
    }

    pub(super) fn allows_function_fields(&self, trait_binding: BindingId) -> bool {
        self.derive_name_is_one_of(trait_binding, &FunctionFieldTraits::ALLOWED)
    }

    pub(super) fn written_impl(
        &self,
        trait_binding: BindingId,
        head: BindingId,
    ) -> Option<&'a TypedTraitImpl> {
        self.traits.impls.iter().find(|implementation| {
            implementation.trait_binding == trait_binding
                && matches!(implementation.implementor, Type::Named(binding, _) if binding == head)
        })
    }

    pub(super) fn select(&self, bound: &Bound, implementor: &Type) -> Selection {
        self.traits.table.select(bound, implementor)
    }

    fn derive_name_is_one_of(&self, trait_binding: BindingId, names: &[&str]) -> bool {
        self.traits
            .table
            .get(trait_binding)
            .is_some_and(|rust_trait| names.contains(&StructDerives::derive_name(&rust_trait.path)))
    }
}
