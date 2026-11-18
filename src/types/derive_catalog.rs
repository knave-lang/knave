use super::derivable_trait::DerivableTrait;
use super::derived_trait::DerivedTrait;
use super::structural_trait::StructuralTrait;
use super::written_derive::WrittenDerive;
use crate::core::{BindingArena, BindingId, CoreModule, ItemKind, StructDerives};
use crate::modules::ModuleSources;
use crate::source::Span;

pub(super) struct DeriveCatalog {
    traits: Vec<DerivableTrait>,
}

impl DeriveCatalog {
    pub(super) fn of(modules: &[CoreModule], structural: &[StructuralTrait]) -> Self {
        let prelude_traits: Vec<(BindingId, &str)> = modules
            .iter()
            .filter(|module| ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::RustTrait(declaration) => {
                    Some((declaration.binding, declaration.path.as_ref()))
                }
                _ => None,
            })
            .collect();
        let written = StructDerives::DERIVABLE.iter().flat_map(|derived| {
            prelude_traits
                .iter()
                .filter(move |(_, path)| StructDerives::derive_name(path) == *derived)
                .map(move |(binding, _)| DerivableTrait {
                    binding: *binding,
                    derived: DerivedTrait::Written(derived),
                })
        });
        let structural = structural.iter().map(|structural| DerivableTrait {
            binding: structural.binding,
            derived: DerivedTrait::Structural,
        });
        Self {
            traits: written.chain(structural).collect(),
        }
    }

    pub(super) fn entries(&self) -> &[DerivableTrait] {
        &self.traits
    }

    pub(super) fn accepts(&self, trait_binding: BindingId) -> bool {
        self.traits
            .iter()
            .any(|entry| entry.binding == trait_binding)
    }

    pub(super) fn names(&self, bindings: &BindingArena) -> Vec<String> {
        self.traits
            .iter()
            .map(|entry| bindings.get(entry.binding).name().to_string())
            .collect()
    }

    pub(super) fn listing_span(
        &self,
        derive: &WrittenDerive,
        entry: DerivableTrait,
    ) -> Option<Span> {
        derive
            .traits
            .iter()
            .find(|(listed, _)| *listed == entry.binding || self.brings(*listed, entry.derived))
            .map(|(_, span)| *span)
    }

    pub(super) fn written_name(&self, trait_binding: BindingId) -> Option<&'static str> {
        self.traits
            .iter()
            .find(|entry| entry.binding == trait_binding)
            .and_then(|entry| match entry.derived {
                DerivedTrait::Written(name) => Some(name),
                DerivedTrait::Structural => None,
            })
    }

    fn brings(&self, listed: BindingId, derived: DerivedTrait) -> bool {
        let listed = self
            .traits
            .iter()
            .find(|entry| entry.binding == listed)
            .map(|entry| entry.derived);
        match (listed, derived) {
            (Some(DerivedTrait::Written(listed)), DerivedTrait::Written(name)) => {
                StructDerives::brings(listed, name)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::source::{SourceId, Span};
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::derivable_trait::DerivableTrait;
    use crate::types::derived_trait::DerivedTrait;
    use crate::types::written_derive::WrittenDerive;

    const SOURCE: &str = "(struct point)\n(defn main () (do))";

    struct CatalogFixture {
        declarations: DeclarationsFixture,
    }

    impl CatalogFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn span(start: usize) -> Span {
            Span::new(SourceId::new(0), start..start + 1)
        }

        fn listing(&self, trait_name: &str) -> WrittenDerive {
            WrittenDerive {
                span: Self::span(0),
                traits: vec![(self.declarations.trait_named(trait_name), Self::span(7))],
            }
        }
    }

    #[test]
    fn a_derivable_trait_and_a_structural_trait_are_accepted() {
        let fixture = CatalogFixture::new();
        let catalog = fixture.declarations.derive_catalog();
        assert!(catalog.accepts(fixture.declarations.trait_named("debug")));
        assert!(catalog.accepts(fixture.declarations.trait_named("show")));
        assert!(!catalog.accepts(fixture.declarations.trait_named("display")));
    }

    #[test]
    fn only_a_written_derive_has_a_derive_name() {
        let fixture = CatalogFixture::new();
        let catalog = fixture.declarations.derive_catalog();
        let debug = fixture.declarations.trait_named("debug");
        let show = fixture.declarations.trait_named("show");
        assert_eq!(catalog.written_name(debug), Some("Debug"));
        assert_eq!(catalog.written_name(show), None);
    }

    #[test]
    fn the_names_list_every_derivable_trait() {
        let fixture = CatalogFixture::new();
        let catalog = fixture.declarations.derive_catalog();
        let names = catalog.names(fixture.declarations.program_bindings());
        assert!(names.iter().any(|name| name == "debug"));
        assert!(names.iter().any(|name| name == "show"));
    }

    #[test]
    fn a_listed_trait_is_found_at_its_own_span() {
        let fixture = CatalogFixture::new();
        let catalog = fixture.declarations.derive_catalog();
        let debug = fixture.declarations.trait_named("debug");
        let listing = fixture.listing("debug");
        let span = catalog.listing_span(
            &listing,
            DerivableTrait {
                binding: debug,
                derived: DerivedTrait::Written("Debug"),
            },
        );
        assert_eq!(span, Some(CatalogFixture::span(7)));
    }

    #[test]
    fn a_listed_trait_brings_the_traits_it_requires() {
        let fixture = CatalogFixture::new();
        let catalog = fixture.declarations.derive_catalog();
        let partial = fixture.declarations.trait_named("partial-eq");
        let listing = fixture.listing("eq");
        let span = catalog.listing_span(
            &listing,
            DerivableTrait {
                binding: partial,
                derived: DerivedTrait::Written("PartialEq"),
            },
        );
        assert_eq!(span, Some(CatalogFixture::span(7)));
    }

    #[test]
    fn an_unrelated_listing_lists_nothing() {
        let fixture = CatalogFixture::new();
        let catalog = fixture.declarations.derive_catalog();
        let debug = fixture.declarations.trait_named("debug");
        let listing = fixture.listing("hash");
        assert_eq!(
            catalog.listing_span(
                &listing,
                DerivableTrait {
                    binding: debug,
                    derived: DerivedTrait::Written("Debug")
                }
            ),
            None
        );
    }
}
