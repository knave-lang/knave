use rustc_hash::FxHashMap;

use super::derivable_trait::DerivableTrait;
use super::derive_catalog::DeriveCatalog;
use super::derive_source::DeriveSource;
use super::written_derive::WrittenDerive;
use crate::core::BindingId;

pub(super) struct DeriveListings<'a> {
    catalog: &'a DeriveCatalog,
    written: FxHashMap<BindingId, WrittenDerive>,
}

impl<'a> DeriveListings<'a> {
    pub(super) fn new(
        catalog: &'a DeriveCatalog,
        written: FxHashMap<BindingId, WrittenDerive>,
    ) -> Self {
        Self { catalog, written }
    }

    pub(super) fn lists(&self, binding: BindingId) -> bool {
        self.written.contains_key(&binding)
    }

    pub(super) fn listed_source(
        &self,
        binding: BindingId,
        entry: DerivableTrait,
    ) -> Option<DeriveSource> {
        let derive = self.written.get(&binding)?;
        self.catalog
            .listing_span(derive, entry)
            .map(DeriveSource::Listed)
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::DeriveListings;
    use crate::source::{SourceId, Span};
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::derivable_trait::DerivableTrait;
    use crate::types::derive_catalog::DeriveCatalog;
    use crate::types::derive_source::DeriveSource;
    use crate::types::derived_trait::DerivedTrait;
    use crate::types::written_derive::WrittenDerive;

    const SOURCE: &str = "(struct point)\n(struct depot)\n(defn main () (do))";
    const LISTED_AT: usize = 7;

    struct ListingsFixture {
        declarations: DeclarationsFixture,
        catalog: DeriveCatalog,
    }

    impl ListingsFixture {
        fn new() -> Self {
            let declarations = DeclarationsFixture::new(SOURCE);
            let catalog = declarations.derive_catalog();
            Self {
                declarations,
                catalog,
            }
        }

        fn listing_debug_on_point(&self) -> DeriveListings<'_> {
            let span = Span::new(SourceId::new(0), LISTED_AT..LISTED_AT + 1);
            let derive = WrittenDerive {
                span,
                traits: vec![(self.declarations.trait_named("debug"), span)],
            };
            let point = self.declarations.type_named("point");
            DeriveListings::new(&self.catalog, FxHashMap::from_iter([(point, derive)]))
        }

        fn entry(&self, trait_name: &str, name: &'static str) -> DerivableTrait {
            DerivableTrait {
                binding: self.declarations.trait_named(trait_name),
                derived: DerivedTrait::Written(name),
            }
        }
    }

    #[test]
    fn only_a_type_with_a_written_derive_is_listed() {
        let fixture = ListingsFixture::new();
        let listings = fixture.listing_debug_on_point();
        assert!(listings.lists(fixture.declarations.type_named("point")));
        assert!(!listings.lists(fixture.declarations.type_named("depot")));
    }

    #[test]
    fn a_listed_trait_has_a_listed_source() {
        let fixture = ListingsFixture::new();
        let listings = fixture.listing_debug_on_point();
        let point = fixture.declarations.type_named("point");
        let source = listings.listed_source(point, fixture.entry("debug", "Debug"));
        let listed = Span::new(SourceId::new(0), LISTED_AT..LISTED_AT + 1);
        assert!(matches!(source, Some(DeriveSource::Listed(span)) if span == listed));
    }

    #[test]
    fn a_trait_the_listing_leaves_out_has_no_source() {
        let fixture = ListingsFixture::new();
        let listings = fixture.listing_debug_on_point();
        let point = fixture.declarations.type_named("point");
        assert!(
            listings
                .listed_source(point, fixture.entry("clone", "Clone"))
                .is_none()
        );
    }
}
