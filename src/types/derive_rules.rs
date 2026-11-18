use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::declarations::Declarations;
use super::declared_traits::DeclaredTraits;
use super::derive_blockers::DeriveBlockers;
use super::derive_source::DeriveSource;
use super::derived_trait::DerivedTrait;
use super::ty::Type;
use super::type_declarations::TypeDeclarations;
use super::variant_payloads::VariantPayloads;
use crate::core::{BindingId, StructDerives};

pub(super) struct DeriveRules<'a> {
    traits: DeclaredTraits<'a>,
    types: &'a TypeDeclarations,
    blockers: DeriveBlockers<'a>,
    changed_structs: &'a FxHashSet<BindingId>,
}

impl<'a> DeriveRules<'a> {
    pub(super) fn new(
        declarations: &'a Declarations,
        changed_structs: &'a FxHashSet<BindingId>,
    ) -> Self {
        Self {
            traits: DeclaredTraits::new(&declarations.traits),
            types: &declarations.types,
            blockers: DeriveBlockers::new(declarations, VariantPayloads::new(&declarations.types)),
            changed_structs,
        }
    }

    pub(super) fn is_rust_enum(&self, binding: BindingId) -> bool {
        self.types
            .enums
            .get(binding)
            .is_some_and(|definition| definition.rust.is_some())
    }

    pub(super) fn blocking_field(
        &self,
        struct_binding: BindingId,
        trait_binding: BindingId,
    ) -> Option<(Rc<str>, Type)> {
        self.blockers.blocking_field(struct_binding, trait_binding)
    }

    pub(super) fn blocking_enum_payload(
        &self,
        binding: BindingId,
        trait_binding: BindingId,
    ) -> Option<(Rc<str>, Type)> {
        self.blockers.blocking_enum_payload(binding, trait_binding)
    }

    pub(super) fn unlisted_source(
        &self,
        struct_binding: BindingId,
        trait_binding: BindingId,
        derived: DerivedTrait,
    ) -> Option<DeriveSource> {
        match derived {
            DerivedTrait::Structural => self
                .traits
                .written_impl(trait_binding, struct_binding)
                .is_none()
                .then_some(DeriveSource::Inferred),
            DerivedTrait::Written(name)
                if name == StructDerives::COPY
                    && self.changed_structs.contains(&struct_binding) =>
            {
                None
            }
            DerivedTrait::Written(name) => StructDerives::INFERRED
                .contains(&name)
                .then_some(DeriveSource::Inferred),
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::derive_source::DeriveSource;
    use crate::types::derived_trait::DerivedTrait;

    const SOURCE: &str = "(struct point)\n(struct depot)\n(defn main () (do))";

    struct RulesFixture {
        declarations: DeclarationsFixture,
        changed: FxHashSet<crate::core::BindingId>,
    }

    impl RulesFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
                changed: FxHashSet::default(),
            }
        }

        fn changing(mut self, name: &str) -> Self {
            self.changed.insert(self.declarations.type_named(name));
            self
        }

        fn inferred(&self, name: &str, trait_name: &str, derived: DerivedTrait) -> bool {
            let binding = self.declarations.type_named(name);
            let trait_binding = self.declarations.trait_named(trait_name);
            matches!(
                self.declarations
                    .derive_rules(&self.changed)
                    .unlisted_source(binding, trait_binding, derived),
                Some(DeriveSource::Inferred)
            )
        }
    }

    #[test]
    fn copy_is_inferred_for_a_struct_nothing_changes() {
        let fixture = RulesFixture::new();
        assert!(fixture.inferred("point", "copy", DerivedTrait::Written("Copy")));
    }

    #[test]
    fn copy_is_not_inferred_for_a_changed_struct() {
        let fixture = RulesFixture::new().changing("point");
        assert!(!fixture.inferred("point", "copy", DerivedTrait::Written("Copy")));
        assert!(fixture.inferred("point", "clone", DerivedTrait::Written("Clone")));
    }

    #[test]
    fn debug_is_never_inferred() {
        let fixture = RulesFixture::new();
        assert!(!fixture.inferred("point", "debug", DerivedTrait::Written("Debug")));
    }

    #[test]
    fn a_structural_trait_is_inferred_without_a_written_impl() {
        let fixture = RulesFixture::new();
        assert!(fixture.inferred("depot", "show", DerivedTrait::Structural));
    }

    #[test]
    fn a_plain_struct_is_no_rust_enum() {
        let fixture = RulesFixture::new();
        let point = fixture.declarations.type_named("point");
        assert!(
            !fixture
                .declarations
                .derive_rules(&fixture.changed)
                .is_rust_enum(point)
        );
    }
}
