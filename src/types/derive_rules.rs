use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::declarations::Declarations;
use super::declared_traits::DeclaredTraits;
use super::definition_kind::DefinitionKind;
use super::derivable_trait::DerivableTrait;
use super::derive_blockers::DeriveBlockers;
use super::derive_candidate::DeriveCandidate;
use super::derive_listings::DeriveListings;
use super::derive_source::DeriveSource;
use super::derived_trait::DerivedTrait;
use super::ty::Type;
use crate::core::{BindingId, StructDerives};

pub(super) struct DeriveRules<'a> {
    traits: DeclaredTraits<'a>,
    blockers: DeriveBlockers<'a>,
    changed_structs: &'a FxHashSet<BindingId>,
}

impl<'a> DeriveRules<'a> {
    pub(super) fn new(
        declarations: &'a Declarations,
        blockers: DeriveBlockers<'a>,
        changed_structs: &'a FxHashSet<BindingId>,
    ) -> Self {
        Self {
            traits: DeclaredTraits::new(declarations.traits()),
            blockers,
            changed_structs,
        }
    }

    pub(super) fn source(
        &self,
        binding: BindingId,
        entry: DerivableTrait,
        listings: &DeriveListings,
    ) -> Option<DeriveSource> {
        if listings.lists(binding) {
            listings.listed_source(binding, entry)
        } else {
            self.unlisted_source(binding, entry)
        }
    }

    pub(super) fn blocked(
        &self,
        candidates: &[DeriveCandidate],
    ) -> Vec<(DeriveCandidate, Rc<str>, Type)> {
        candidates
            .iter()
            .filter_map(|candidate| {
                self.blocking(candidate)
                    .map(|(field, ty)| (*candidate, field, ty))
            })
            .collect()
    }

    pub(super) fn blocking(&self, candidate: &DeriveCandidate) -> Option<(Rc<str>, Type)> {
        match candidate.kind {
            DefinitionKind::Struct => {
                self.blocking_field(candidate.binding, candidate.trait_binding)
            }
            DefinitionKind::Enum => {
                self.blocking_enum_payload(candidate.binding, candidate.trait_binding)
            }
        }
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

    fn unlisted_source(
        &self,
        struct_binding: BindingId,
        entry: DerivableTrait,
    ) -> Option<DeriveSource> {
        let trait_binding = entry.binding;
        match entry.derived {
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
    use rustc_hash::{FxHashMap, FxHashSet};

    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::definition_kind::DefinitionKind;
    use crate::types::derivable_trait::DerivableTrait;
    use crate::types::derive_candidate::DeriveCandidate;
    use crate::types::derive_listings::DeriveListings;
    use crate::types::derive_source::DeriveSource;
    use crate::types::derived_trait::DerivedTrait;

    const SOURCE: &str = "
        (struct point)
        (struct depot)
        (struct ticket (id u32) (owner string))
        (enum reading (text string) (number f64))
        (defn main () (do))";

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

        fn candidate(&self, name: &str, kind: DefinitionKind, trait_name: &str) -> DeriveCandidate {
            DeriveCandidate {
                binding: self.declarations.type_named(name),
                kind,
                trait_binding: self.declarations.trait_named(trait_name),
                source: DeriveSource::Inferred,
            }
        }

        fn blocking(&self, candidate: &DeriveCandidate) -> Option<String> {
            self.declarations
                .derive_rules(&self.changed)
                .blocking(candidate)
                .map(|(field, _)| field.to_string())
        }

        fn inferred(&self, name: &str, trait_name: &str, derived: DerivedTrait) -> bool {
            let binding = self.declarations.type_named(name);
            let trait_binding = self.declarations.trait_named(trait_name);
            let catalog = self.declarations.derive_catalog();
            let unlisted = DeriveListings::new(&catalog, FxHashMap::default());
            let entry = DerivableTrait {
                binding: trait_binding,
                derived,
            };
            matches!(
                self.declarations
                    .derive_rules(&self.changed)
                    .source(binding, entry, &unlisted),
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
    fn a_struct_is_blocked_by_its_field() {
        let fixture = RulesFixture::new();
        let ticket = fixture.candidate("ticket", DefinitionKind::Struct, "copy");
        assert_eq!(fixture.blocking(&ticket), Some("owner".to_string()));
    }

    #[test]
    fn an_enum_is_blocked_by_its_payload() {
        let fixture = RulesFixture::new();
        let reading = fixture.candidate("reading", DefinitionKind::Enum, "copy");
        assert_eq!(fixture.blocking(&reading), Some("text".to_string()));
    }

    #[test]
    fn only_the_blocked_candidates_are_listed() {
        let fixture = RulesFixture::new();
        let candidates = [
            fixture.candidate("ticket", DefinitionKind::Struct, "clone"),
            fixture.candidate("ticket", DefinitionKind::Struct, "copy"),
        ];
        let blocked = fixture
            .declarations
            .derive_rules(&fixture.changed)
            .blocked(&candidates);
        let traits: Vec<_> = blocked
            .iter()
            .map(|(candidate, field, _)| (candidate.trait_binding, field.to_string()))
            .collect();
        let copy = fixture.declarations.trait_named("copy");
        assert_eq!(traits, vec![(copy, "owner".to_string())]);
    }
}
