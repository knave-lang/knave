use std::rc::Rc;

#[cfg(test)]
use super::imported_counts::ImportedCounts;
use super::imported_table::ImportedTable;
use super::name_entries::NameEntries;
use super::name_key::NameKey;
use super::qualified_names::QualifiedNames;
use super::saved_exports::SavedExports;
use super::{Scope, ScopeSet};
use crate::core::{BindingId, ModuleId, Namespace};

#[derive(Default)]
pub(super) struct ImportedNames {
    tables: Vec<ImportedTable>,
    qualified_names: QualifiedNames,
}

impl ImportedNames {
    pub(super) fn add(&mut self, scope: Scope, base: ModuleId, exports: SavedExports) {
        let table = match self.tables.iter().position(|table| table.is_of(base)) {
            Some(table) => table,
            None => self.add_table(base, exports),
        };
        self.tables[table].add(scope);
    }

    pub(super) fn visit_entries_seen_from<'t>(
        &'t self,
        scopes: &ScopeSet,
        key: NameKey<'_>,
        visit: &mut impl FnMut(&'t NameEntries),
    ) {
        for table in &self.tables {
            table.visit_entries_seen_from(scopes, key, visit);
        }
    }

    pub(super) fn bound_with_scopes(
        &self,
        scopes: &ScopeSet,
        key: NameKey<'_>,
    ) -> Option<BindingId> {
        if scopes.len() != 1 {
            return None;
        }
        let scope = scopes.largest_matching(|_| true)?;
        self.tables
            .iter()
            .find_map(|table| table.bound_with_scope(scope, key))
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        self.tables.iter().flat_map(ImportedTable::all)
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        self.tables
            .iter()
            .flat_map(move |table| table.all_of(namespace))
    }

    pub(super) fn qualified_ending_in(
        &self,
        last_segment: Rc<str>,
        namespace: Namespace,
    ) -> &[Rc<str>] {
        self.qualified_names.ending_in(last_segment, namespace)
    }

    #[cfg(test)]
    pub(super) fn counts(&self) -> ImportedCounts {
        ImportedCounts {
            tables: self.tables.len(),
            modules: self.tables.iter().map(ImportedTable::module_count).sum(),
            decoded_names: self.tables.iter().map(ImportedTable::decoded_count).sum(),
        }
    }

    fn add_table(&mut self, base: ModuleId, exports: SavedExports) -> usize {
        for position in 0..exports.count() {
            let (name, namespace, _) = exports.export(position);
            self.qualified_names.record_saved(name, namespace);
        }
        self.tables.push(ImportedTable::new(base, exports));
        self.tables.len() - 1
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::rc::Rc;

    use crate::core::{BindingId, ModuleId, Namespace};
    use crate::reader::Symbol;
    use crate::scope::binding_table::tests::World;
    use crate::scope::{ExportBytes, Phase, Resolution, SavedExports, Scope, ScopeSet};

    pub(in crate::scope) struct ImportFixture {
        pub(in crate::scope) world: World,
        pub(in crate::scope) module: Scope,
    }

    impl ImportFixture {
        pub(in crate::scope) const MAP: BindingId = BindingId::from_index(3);
        const SOME: BindingId = BindingId::from_index(5);
        const VEC: BindingId = BindingId::from_index(8);
        const FILTER: BindingId = BindingId::from_index(11);

        pub(in crate::scope) fn new() -> Self {
            let mut world = World::new();
            let module = world.scope();
            world
                .table
                .import_prelude(module, Self::base(), Self::exports());
            Self { world, module }
        }

        fn base() -> ModuleId {
            ModuleId::new(0)
        }

        fn exports() -> SavedExports {
            Self::saved(&[
                ("map", Namespace::Value, Self::MAP),
                ("option/some", Namespace::Value, Self::SOME),
                ("vec", Namespace::Type, Self::VEC),
            ])
        }

        fn kernel_exports() -> SavedExports {
            Self::saved(&[("filter", Namespace::Value, Self::FILTER)])
        }

        fn saved(exports: &[(&str, Namespace, BindingId)]) -> SavedExports {
            let bytes = bincode::serialize(&ExportBytes::of(exports.iter().copied()))
                .expect("the exports save");
            bincode::deserialize(bytes.leak()).expect("the exports load")
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

        fn import_into_new_module(&mut self) -> Scope {
            let module = self.world.scope();
            self.world
                .table
                .import_prelude(module, Self::base(), Self::exports());
            module
        }

        fn decoded_count(&self) -> usize {
            self.world.table.imported_prelude_counts().decoded_names
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
    fn a_name_resolves_under_its_own_module_among_more_imports_than_scopes() {
        let mut fixture = ImportFixture::new();
        let second = fixture.import_into_new_module();
        fixture.import_into_new_module();
        let elsewhere = fixture.world.scope();
        assert_eq!(
            fixture.resolve("map", &[second]),
            Resolution::Bound(ImportFixture::MAP)
        );
        assert_eq!(fixture.resolve("map", &[elsewhere]), Resolution::Unbound);
        assert_eq!(fixture.decoded_count(), 1);
    }

    #[test]
    fn an_own_entry_that_ties_with_a_shared_entry_takes_the_mark() {
        let mut fixture = ImportFixture::new();
        let beside = fixture.world.scope();
        let table = &mut fixture.world.table;
        let own = World::symbol("map", &[beside]);
        let bound = table.bind(&own, Namespace::Value, ImportFixture::MAP, Phase::Runtime);
        assert_eq!(bound, Ok(()));
        let written = World::symbol("map", &[fixture.module, beside]);
        table.resolve_use(&written, Namespace::Value, Phase::Runtime);
        let marked =
            table
                .visible_names()
                .use_of(Rc::from("map"), own.scopes(), ImportFixture::MAP);
        assert_eq!(marked, Some(true));
    }

    #[test]
    fn a_written_use_of_an_imported_name_holds_its_own_mark() {
        let mut fixture = ImportFixture::new();
        let body = fixture.world.scope();
        let other_module = fixture.import_into_new_module();
        let written = World::symbol("map", &[fixture.module, body]);
        let table = &mut fixture.world.table;
        let bound = table.bind(
            &written,
            Namespace::Value,
            ImportFixture::MAP,
            Phase::Runtime,
        );
        assert_eq!(bound, Ok(()));
        table.resolve_use(&written, Namespace::Value, Phase::Runtime);
        let names = table.visible_names();
        let marked = |scopes: &[Scope]| {
            let scopes = World::symbol("map", scopes).scopes().clone();
            names.use_of(Rc::from("map"), &scopes, ImportFixture::MAP)
        };
        assert_eq!(marked(&[fixture.module, body]), Some(true));
        assert_eq!(marked(&[other_module]), Some(false));
    }

    #[test]
    fn a_binding_under_a_scope_no_module_imports_does_not_clash() {
        let mut fixture = ImportFixture::new();
        let elsewhere = fixture.world.scope();
        let symbol = World::symbol("map", &[elsewhere]);
        let other = BindingId::from_index(9);
        let bound = fixture
            .world
            .table
            .bind(&symbol, Namespace::Value, other, Phase::Runtime);
        assert_eq!(bound, Ok(()));
    }

    #[test]
    fn a_binding_under_more_than_the_import_scopes_does_not_clash() {
        let mut fixture = ImportFixture::new();
        let later = fixture.import_into_new_module();
        let symbol = World::symbol("map", &[fixture.module, later]);
        let other = BindingId::from_index(9);
        let bound = fixture
            .world
            .table
            .bind(&symbol, Namespace::Value, other, Phase::Runtime);
        assert_eq!(bound, Ok(()));
    }

    #[test]
    fn imports_of_one_base_share_its_table_and_another_base_adds_one() {
        let mut fixture = ImportFixture::new();
        let second = fixture.import_into_new_module();
        let other = fixture.world.scope();
        let table = &mut fixture.world.table;
        table.import_prelude(other, ModuleId::new(1), ImportFixture::exports());
        let counts = table.imported_prelude_counts();
        assert_eq!((counts.tables, counts.modules), (2, 3));
        assert_eq!(
            fixture.resolve("map", &[second]),
            Resolution::Bound(ImportFixture::MAP)
        );
    }

    #[test]
    fn a_module_sees_only_the_exports_of_its_own_base() {
        let mut fixture = ImportFixture::new();
        let kernel_user = fixture.world.scope();
        let kernel = ModuleId::new(1);
        fixture
            .world
            .table
            .import_prelude(kernel_user, kernel, ImportFixture::kernel_exports());
        assert_eq!(
            fixture.resolve("filter", &[kernel_user]),
            Resolution::Bound(ImportFixture::FILTER)
        );
        assert_eq!(fixture.resolve("map", &[kernel_user]), Resolution::Unbound);
        assert_eq!(
            fixture.resolve("filter", &[fixture.module]),
            Resolution::Unbound
        );
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
            names
                .qualified_entries_ending_in(&last_segment, Namespace::Value, Phase::Runtime)
                .bindings(),
            [ImportFixture::SOME]
        );
        assert!(
            names
                .qualified_entries_ending_in(&map, Namespace::Value, Phase::Runtime)
                .bindings()
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
