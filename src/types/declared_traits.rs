use super::selection::Selection;
use super::trait_declarations::TraitDeclarations;
use super::ty::{Bound, Type};
use super::typed_trait_impl::TypedTraitImpl;
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
    fn a_trait_rust_derives_is_derivable() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let debug = fixture.trait_named("debug");
        assert!(fixture.declared_traits().is_derivable(debug));
    }

    #[test]
    fn a_trait_rust_cannot_derive_is_not_derivable() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let display = fixture.trait_named("display");
        assert!(!fixture.declared_traits().is_derivable(display));
    }

    #[test]
    fn a_structural_trait_is_derivable() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let show = fixture.trait_named("show");
        assert!(fixture.declared_traits().is_derivable(show));
    }

    #[test]
    fn only_a_trait_with_a_structural_clause_is_structural() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let traits = fixture.declared_traits();
        assert!(traits.is_structural(fixture.trait_named("show")));
        assert!(!traits.is_structural(fixture.trait_named("debug")));
    }

    #[test]
    fn debug_allows_function_fields_and_equality_does_not() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let traits = fixture.declared_traits();
        assert!(traits.allows_function_fields(fixture.trait_named("debug")));
        assert!(!traits.allows_function_fields(fixture.trait_named("eq")));
    }

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
