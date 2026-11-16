use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ScopeSet;
use super::largest_candidates::LargestCandidates;
use super::name_entries::NameEntries;
use super::name_entry::NameEntry;
use super::name_key::NameKey;
use super::phase::Phase;
use super::prelude_names::PreludeNames;
use super::qualified_entries::QualifiedEntries;
use super::qualified_names::QualifiedNames;
use crate::core::{BindingId, Namespace};
use crate::reader::Symbol;

pub struct VisibleNames<'t> {
    entries: &'t FxHashMap<(Rc<str>, Namespace), NameEntries>,
    prelude: &'t PreludeNames,
    qualified_names: &'t QualifiedNames,
}

impl<'t> VisibleNames<'t> {
    #[inline(always)]
    pub(super) fn new(
        entries: &'t FxHashMap<(Rc<str>, Namespace), NameEntries>,
        prelude: &'t PreludeNames,
        qualified_names: &'t QualifiedNames,
    ) -> Self {
        Self {
            entries,
            prelude,
            qualified_names,
        }
    }

    pub fn name_bindings(
        &self,
        symbol: &Symbol,
        namespace: Namespace,
        phase: Phase,
    ) -> Vec<(Rc<str>, BindingId)> {
        let own = self
            .entries
            .iter()
            .map(|((name, entry_namespace), entries)| (Rc::clone(name), *entry_namespace, entries));
        own.chain(self.prelude.all())
            .filter(|(_, entry_namespace, _)| *entry_namespace == namespace)
            .flat_map(|(name, _, entries)| {
                entries
                    .visible(symbol.scopes(), phase)
                    .into_iter()
                    .map(move |entry| (Rc::clone(&name), entry.binding()))
            })
            .collect()
    }

    pub fn bindings(
        &self,
        scopes: &ScopeSet,
        namespace: Namespace,
        phase: Phase,
    ) -> Vec<BindingId> {
        let own = self
            .entries
            .iter()
            .filter(|((_, entry_namespace), _)| *entry_namespace == namespace)
            .map(|(_, entries)| entries);
        own.chain(self.prelude.all_of(namespace))
            .flat_map(|entries| entries.visible(scopes, phase))
            .map(|entry| entry.binding())
            .collect()
    }

    pub fn qualified_entries_ending_in(
        &self,
        last_segment: &Symbol,
        namespace: Namespace,
        phase: Phase,
    ) -> QualifiedEntries<'t> {
        QualifiedEntries::new(self.qualified_entries(last_segment, namespace, phase))
    }

    #[inline(always)]
    pub fn largest_candidates(
        &self,
        symbol: &Symbol,
        namespace: Namespace,
        phase: Phase,
    ) -> LargestCandidates<'t> {
        let mut largest = LargestCandidates::default();
        let key = (symbol.shared_name(), namespace);
        self.visit_entries_named(symbol.scopes(), &key, &mut |entries| {
            entries.visit_visible(symbol.scopes(), phase, |entry| largest.consider(entry));
        });
        largest
    }

    pub fn hides_a_macro(
        &self,
        symbol: &Symbol,
        phase: Phase,
        is_macro: impl Fn(BindingId) -> bool,
    ) -> bool {
        let mut visible = Vec::new();
        let key = (symbol.shared_name(), Namespace::Value);
        self.visit_entries_named(symbol.scopes(), &key, &mut |entries| {
            entries.visit_visible(symbol.scopes(), phase, |entry| {
                visible.push((entry.scopes().len(), entry.binding()));
            });
        });
        let Some(&(largest, winner)) = visible.iter().max_by_key(|(size, _)| *size) else {
            return false;
        };
        visible
            .iter()
            .any(|&(size, binding)| size < largest && binding != winner && is_macro(binding))
    }

    pub fn use_of(&self, name: Rc<str>, scopes: &ScopeSet, binding: BindingId) -> Option<bool> {
        let mut used = None;
        for namespace in Namespace::IMPORTED {
            if used == Some(true) {
                break;
            }
            let key = (Rc::clone(&name), namespace);
            self.visit_entries_named(scopes, &key, &mut |entries| {
                entries.visit_visible(scopes, Phase::CompileTime, |entry| {
                    if entry.binding() == binding {
                        let marked = entry.use_mark().is_marked();
                        used = Some(used.unwrap_or(false) || marked);
                    }
                });
            });
        }
        used
    }

    fn qualified_entries(
        &self,
        last_segment: &Symbol,
        namespace: Namespace,
        phase: Phase,
    ) -> Vec<&'t NameEntry> {
        let mut visible = Vec::new();
        let own = self
            .qualified_names
            .ending_in(last_segment.shared_name(), namespace);
        let prelude = self
            .prelude
            .qualified_ending_in(last_segment.shared_name(), namespace);
        for name in own.iter().chain(prelude) {
            let key = (Rc::clone(name), namespace);
            self.visit_entries_named(last_segment.scopes(), &key, &mut |entries| {
                entries.visit_visible(last_segment.scopes(), phase, |entry| visible.push(entry));
            });
        }
        visible
    }

    #[inline(always)]
    fn visit_entries_named(
        &self,
        scopes: &ScopeSet,
        key: &(Rc<str>, Namespace),
        visit: &mut impl FnMut(&'t NameEntries),
    ) {
        if let Some(own) = self.entries.get(key) {
            visit(own);
        }
        let (name, namespace) = key;
        let key = NameKey::new(name, *namespace);
        self.prelude.visit_entries_seen_from(scopes, key, visit);
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::super::binding_table::tests::World;
    use super::super::imported_names::tests::ImportFixture;
    use super::super::{Phase, Scope, ScopeSet};
    use crate::core::{BindingId, Namespace};

    struct NamesFixture {
        world: World,
    }

    impl NamesFixture {
        fn new() -> Self {
            Self {
                world: World::new(),
            }
        }

        fn world(&mut self) -> &mut World {
            &mut self.world
        }

        fn name_bindings(
            &self,
            scopes: &[Scope],
            namespace: Namespace,
            phase: Phase,
        ) -> Vec<(Rc<str>, BindingId)> {
            let symbol = World::symbol("anything", scopes);
            let mut listed = self
                .world
                .table
                .visible_names()
                .name_bindings(&symbol, namespace, phase);
            listed.sort_by_key(|(_, binding)| *binding);
            listed
        }

        fn visible(&self, scopes: &[Scope], namespace: Namespace, phase: Phase) -> Vec<BindingId> {
            let mut visible =
                self.world
                    .table
                    .visible_names()
                    .bindings(&ScopeSet::of(scopes), namespace, phase);
            visible.sort();
            visible
        }

        fn qualified_ending_in(
            &self,
            last_segment: &str,
            scopes: &[Scope],
            phase: Phase,
        ) -> Vec<BindingId> {
            let mut found = self
                .world
                .table
                .visible_names()
                .qualified_entries_ending_in(
                    &World::symbol(last_segment, scopes),
                    Namespace::Value,
                    phase,
                )
                .bindings();
            found.sort();
            found
        }
    }

    #[test]
    fn a_saved_type_name_is_listed_in_the_type_namespace_only() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let saved = fixture.world().define_type("saved", &[scope]);
        fixture.world().save_names();
        assert_eq!(
            fixture.name_bindings(&[scope], Namespace::Type, Phase::Runtime),
            [(Rc::from("saved"), saved)]
        );
        assert!(
            fixture
                .name_bindings(&[scope], Namespace::Value, Phase::Runtime)
                .is_empty()
        );
    }

    #[test]
    fn a_saved_name_is_listed_beside_the_programs_own() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let saved = fixture.world().define("saved", &[scope], Phase::Runtime);
        fixture.world().save_names();
        let own = fixture.world().define("own", &[scope], Phase::Runtime);
        assert_eq!(
            fixture.name_bindings(&[scope], Namespace::Value, Phase::Runtime),
            [(Rc::from("saved"), saved), (Rc::from("own"), own)]
        );
    }

    #[test]
    fn a_saved_binding_is_visible_beside_the_programs_own() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let saved = fixture.world().define_type("saved", &[scope]);
        fixture.world().save_names();
        let own = fixture.world().define_type("own", &[scope]);
        assert_eq!(
            fixture.visible(&[scope], Namespace::Type, Phase::Runtime),
            [saved, own]
        );
    }

    #[test]
    fn a_type_lookup_sees_only_the_programs_own_type_bindings() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        fixture.world().define("shape", &[scope], Phase::Runtime);
        let shape_type = fixture.world().define_type("shape", &[scope]);
        assert_eq!(
            fixture.visible(&[scope], Namespace::Type, Phase::Runtime),
            [shape_type]
        );
    }

    #[test]
    fn a_saved_qualified_name_is_found_by_its_last_segment() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let saved = fixture
            .world()
            .define("option/some", &[scope], Phase::Runtime);
        fixture.world().save_names();
        assert_eq!(
            fixture.qualified_ending_in("some", &[scope], Phase::Runtime),
            [saved]
        );
    }

    #[test]
    fn a_qualified_binding_is_found_by_its_last_segment() {
        let mut fixture = NamesFixture::new();
        let module = fixture.world().scope();
        let world = fixture.world();
        let circle = world.define("shape/circle", &[module], Phase::Runtime);
        let nested = world.define("geometry/shape/circle", &[module], Phase::Runtime);
        world.define("circle", &[module], Phase::Runtime);
        world.define("shape/square", &[module], Phase::Runtime);
        assert_eq!(
            fixture.qualified_ending_in("circle", &[module], Phase::Runtime),
            [circle, nested]
        );
    }

    #[test]
    fn a_qualified_binding_outside_the_reference_scopes_is_not_found() {
        let mut fixture = NamesFixture::new();
        let (module, inner) = (fixture.world().scope(), fixture.world().scope());
        fixture
            .world()
            .define("shape/circle", &[module, inner], Phase::Runtime);
        assert_eq!(
            fixture.qualified_ending_in("circle", &[module], Phase::Runtime),
            []
        );
    }

    #[test]
    fn a_qualified_name_in_another_namespace_is_not_found() {
        let mut fixture = NamesFixture::new();
        let module = fixture.world().scope();
        fixture.world().define_type("shape/circle", &[module]);
        assert_eq!(
            fixture.qualified_ending_in("circle", &[module], Phase::Runtime),
            []
        );
    }

    #[test]
    fn a_qualified_name_bound_twice_finds_each_binding_once() {
        let mut fixture = NamesFixture::new();
        let (module, inner) = (fixture.world().scope(), fixture.world().scope());
        let world = fixture.world();
        let outer = world.define("shape/circle", &[module], Phase::Runtime);
        let shadow = world.define("shape/circle", &[module, inner], Phase::Runtime);
        assert_eq!(
            fixture.qualified_ending_in("circle", &[module, inner], Phase::Runtime),
            [outer, shadow]
        );
    }

    #[test]
    fn a_compile_time_name_is_listed_only_for_a_compile_time_lookup() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let helper = fixture
            .world()
            .define("helper", &[scope], Phase::CompileTime);
        assert_eq!(
            fixture.name_bindings(&[scope], Namespace::Value, Phase::CompileTime),
            [(Rc::from("helper"), helper)]
        );
        assert!(
            fixture
                .name_bindings(&[scope], Namespace::Value, Phase::Runtime)
                .is_empty()
        );
    }

    #[test]
    fn a_compile_time_binding_is_visible_only_to_a_compile_time_lookup() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let helper = fixture
            .world()
            .define("helper", &[scope], Phase::CompileTime);
        assert_eq!(
            fixture.visible(&[scope], Namespace::Value, Phase::CompileTime),
            [helper]
        );
        assert!(
            fixture
                .visible(&[scope], Namespace::Value, Phase::Runtime)
                .is_empty()
        );
    }

    #[test]
    fn a_compile_time_qualified_binding_is_found_only_by_a_compile_time_lookup() {
        let mut fixture = NamesFixture::new();
        let scope = fixture.world().scope();
        let helper = fixture
            .world()
            .define("tools/helper", &[scope], Phase::CompileTime);
        assert_eq!(
            fixture.qualified_ending_in("helper", &[scope], Phase::CompileTime),
            [helper]
        );
        assert!(
            fixture
                .qualified_ending_in("helper", &[scope], Phase::Runtime)
                .is_empty()
        );
    }

    #[test]
    fn own_entries_are_visited_before_the_shared_prelude_entries() {
        let mut fixture = ImportFixture::new();
        let body = fixture.world.scope();
        let own = BindingId::from_index(20);
        let written = World::symbol("map", &[fixture.module, body]);
        let table = &mut fixture.world.table;
        let bound = table.bind(&written, Namespace::Value, own, Phase::Runtime);
        assert_eq!(bound, Ok(()));
        let key = (Rc::from("map"), Namespace::Value);
        let mut holds_own = Vec::new();
        table
            .visible_names()
            .visit_entries_named(written.scopes(), &key, &mut |entries| {
                holds_own.push(entries.holds(own));
            });
        assert_eq!(holds_own, [true, false]);
    }
}
