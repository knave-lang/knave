use super::impl_overlap::ImplementorOverlap;
use super::selection::Selection;
use super::trait_declarations::TraitDeclarations;
use super::ty::{Bound, Type};
use super::typed_trait_impl::TypedTraitImpl;
use super::{ReceiverKind, TypedTrait};
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct DeclaredTraits<'a> {
    traits: &'a TraitDeclarations,
}

impl<'a> DeclaredTraits<'a> {
    pub(super) fn new(traits: &'a TraitDeclarations) -> Self {
        Self { traits }
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

    pub(super) fn overlapping_impl(
        &self,
        trait_binding: BindingId,
        implementor: &Type,
    ) -> Option<&'a TypedTraitImpl> {
        self.traits.impls.iter().find(|implementation| {
            implementation.trait_binding == trait_binding
                && ImplementorOverlap::overlaps(&implementation.implementor, implementor)
        })
    }

    pub(super) fn select(&self, bound: &Bound, implementor: &Type) -> Selection {
        self.traits
            .table
            .rows(bound.binding)
            .select(bound, implementor)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "
        (struct point)
        (struct depot)
        (rust-type fmt-result \"std::fmt::Result\")
        (rust-trait shown \"std::fmt::Write\"
          (method write-str
            (fn ((mut self) (ref str)) fmt-result)))
        (impl shown point
          (write-str (p _text)
                     (rust \"std::fmt::Result::Ok(())\" fmt-result)))
        (defn main () (do))";

    #[test]
    fn the_written_impl_of_a_trait_for_a_type_is_found() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let shown = fixture.trait_named("shown");
        let point = fixture.type_named("point");
        let written = fixture.declared_traits().written_impl(shown, point);
        assert_eq!(
            written.map(|implementation| implementation.trait_binding),
            Some(shown)
        );
    }

    #[test]
    fn a_type_without_a_written_impl_has_none() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let shown = fixture.trait_named("shown");
        let depot = fixture.type_named("depot");
        assert!(
            fixture
                .declared_traits()
                .written_impl(shown, depot)
                .is_none()
        );
    }

    #[test]
    fn a_written_impl_of_another_trait_is_not_found() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let debug = fixture.trait_named("debug");
        let point = fixture.type_named("point");
        assert!(
            fixture
                .declared_traits()
                .written_impl(debug, point)
                .is_none()
        );
    }
}
