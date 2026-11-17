use std::cell::OnceCell;
use std::rc::Rc;

use super::ScopeSet;
use super::name_entries::NameEntries;
use super::phase::Phase;
use super::saved_exports::SavedExports;
use crate::core::{BindingId, Namespace};

#[derive(Default)]
pub(super) struct ImportedNames {
    modules: Vec<ImportedModule>,
}

struct ImportedModule {
    scopes: ScopeSet,
    exports: SavedExports,
    entries: Vec<OnceCell<Box<NameEntries>>>,
}

impl ImportedNames {
    pub(super) fn add(&mut self, scopes: ScopeSet, exports: SavedExports) {
        let entries = (0..exports.count()).map(|_| OnceCell::new()).collect();
        self.modules.push(ImportedModule {
            scopes,
            exports,
            entries,
        });
    }

    pub(super) fn entries_seen_from<'t, 's>(
        &'t self,
        scopes: &'s ScopeSet,
        name: Rc<str>,
        namespace: Namespace,
    ) -> impl Iterator<Item = &'t NameEntries> + use<'t, 's> {
        self.modules
            .iter()
            .filter(|module| module.scopes.is_subset_of(scopes))
            .filter_map(move |module| module.entries(&name, namespace))
    }

    pub(super) fn bound_with_scopes(
        &self,
        scopes: &ScopeSet,
        name: &str,
        namespace: Namespace,
    ) -> Option<BindingId> {
        self.modules
            .iter()
            .filter(|module| module.scopes == *scopes)
            .find_map(|module| module.exports.get(name, namespace))
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        self.modules.iter().flat_map(ImportedModule::all)
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        self.modules
            .iter()
            .flat_map(move |module| module.all_of(namespace))
    }

    #[cfg(test)]
    pub(super) fn decoded_count(&self) -> usize {
        let entries = self.modules.iter().flat_map(|module| &module.entries);
        entries.filter(|entry| entry.get().is_some()).count()
    }
}

impl ImportedModule {
    fn entries(&self, name: &str, namespace: Namespace) -> Option<&NameEntries> {
        let position = self.exports.position(name, namespace)?;
        Some(self.decoded(position))
    }

    fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        (0..self.entries.len()).map(|position| {
            let (name, namespace) = self.exports.key(position);
            (Rc::from(name), namespace, self.decoded(position))
        })
    }

    fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        (0..self.entries.len())
            .filter(move |position| self.exports.key(*position).1 == namespace)
            .map(|position| self.decoded(position))
    }

    fn decoded(&self, position: usize) -> &NameEntries {
        self.entries[position].get_or_init(|| Box::new(self.decode(position)))
    }

    fn decode(&self, position: usize) -> NameEntries {
        let mut entries = NameEntries::default();
        entries
            .bind(&self.scopes, self.exports.binding(position), Phase::Runtime)
            .expect("an empty entry list takes any binding");
        entries
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::core::{BindingId, Namespace};
    use crate::reader::Symbol;
    use crate::scope::binding_table::tests::World;
    use crate::scope::{ExportBytes, Phase, Resolution, SavedExports, Scope, ScopeSet};

    struct ImportFixture {
        world: World,
        module: Scope,
    }

    impl ImportFixture {
        const MAP: BindingId = BindingId::from_index(3);
        const SOME: BindingId = BindingId::from_index(5);
        const VEC: BindingId = BindingId::from_index(8);

        fn new() -> Self {
            let mut world = World::new();
            let module = world.scope();
            world
                .table
                .import_saved(ScopeSet::of(&[module]), Self::exports());
            Self { world, module }
        }

        fn exports() -> SavedExports {
            let exports = [
                ("map", Namespace::Value, Self::MAP),
                ("option/some", Namespace::Value, Self::SOME),
                ("vec", Namespace::Type, Self::VEC),
            ];
            let bytes = bincode::serialize(&ExportBytes::of(exports.into_iter()))
                .expect("the exports save");
            SavedExports::of(bincode::deserialize(bytes.leak()).expect("the exports load"))
        }

        fn resolve(&self, name: &str, scopes: &[Scope]) -> Resolution {
            let symbol = World::symbol(name, scopes);
            self.world
                .table
                .resolve(&symbol, Namespace::Value, Phase::Runtime)
        }

        fn bind(&mut self, name: &str, binding: BindingId) -> Result<(), BindingId> {
            let symbol = World::symbol(name, &[self.module]);
            self.world
                .table
                .bind(&symbol, Namespace::Value, binding, Phase::Runtime)
        }

        fn decoded_count(&self) -> usize {
            self.world.table.imported_decoded_count()
        }
    }

    #[test]
    fn an_import_decodes_no_name_until_a_lookup_reaches_it() {
        let fixture = ImportFixture::new();
        assert_eq!(fixture.decoded_count(), 0);
        fixture.resolve("map", &[fixture.module]);
        assert_eq!(fixture.decoded_count(), 1);
    }

    #[test]
    fn an_imported_name_resolves_only_under_the_import_scopes() {
        let mut fixture = ImportFixture::new();
        let elsewhere = fixture.world.scope();
        assert_eq!(
            fixture.resolve("map", &[fixture.module]),
            Resolution::Bound(ImportFixture::MAP)
        );
        assert_eq!(fixture.resolve("map", &[elsewhere]), Resolution::Unbound);
        assert_eq!(fixture.decoded_count(), 1);
    }

    #[test]
    fn a_deeper_binding_shadows_an_imported_name() {
        let mut fixture = ImportFixture::new();
        let body = fixture.world.scope();
        let module = fixture.module;
        let own = fixture.world.define("map", &[module, body], Phase::Runtime);
        assert_eq!(
            fixture.resolve("map", &[module, body]),
            Resolution::Bound(own)
        );
    }

    #[test]
    fn a_binding_under_the_import_scopes_clashes_with_the_imported_name() {
        let mut fixture = ImportFixture::new();
        let other = BindingId::from_index(9);
        assert_eq!(fixture.bind("map", other), Err(ImportFixture::MAP));
        assert_eq!(fixture.bind("map", ImportFixture::MAP), Ok(()));
        assert_eq!(fixture.bind("filter", other), Ok(()));
    }

    #[test]
    fn an_imported_qualified_name_is_found_by_its_last_segment() {
        let fixture = ImportFixture::new();
        let names = fixture.world.table.visible_names();
        let last_segment = World::symbol("some", &[fixture.module]);
        let map = World::symbol("map", &[fixture.module]);
        assert_eq!(
            names.qualified_bindings_ending_in(&last_segment, Namespace::Value, Phase::Runtime),
            [ImportFixture::SOME]
        );
        assert!(
            names
                .qualified_bindings_ending_in(&map, Namespace::Value, Phase::Runtime)
                .is_empty()
        );
    }

    #[test]
    fn a_listing_holds_each_imported_name_of_its_namespace() {
        let fixture = ImportFixture::new();
        let names = fixture.world.table.visible_names();
        let symbol = Symbol::with_scopes("anything", ScopeSet::of(&[fixture.module]));
        let mut listed = names.name_bindings(&symbol, Namespace::Value, Phase::Runtime);
        listed.sort_by_key(|(_, binding)| *binding);
        assert_eq!(
            listed,
            [
                (Rc::from("map"), ImportFixture::MAP),
                (Rc::from("option/some"), ImportFixture::SOME)
            ]
        );
        let types = names.bindings(symbol.scopes(), Namespace::Type, Phase::Runtime);
        assert_eq!(types, [ImportFixture::VEC]);
    }
}
