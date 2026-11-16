use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ScopeSet;
use super::name_entries::NameEntries;
use super::phase::Phase;
use super::qualified_names::QualifiedNames;
use super::saved_names::SavedNames;
use crate::core::{BindingId, Namespace};
use crate::reader::Symbol;

pub struct VisibleNames<'t> {
    entries: &'t FxHashMap<(Rc<str>, Namespace), NameEntries>,
    saved: &'t SavedNames,
    qualified_names: &'t QualifiedNames,
}

impl<'t> VisibleNames<'t> {
    #[inline(always)]
    pub(super) fn new(
        entries: &'t FxHashMap<(Rc<str>, Namespace), NameEntries>,
        saved: &'t SavedNames,
        qualified_names: &'t QualifiedNames,
    ) -> Self {
        Self {
            entries,
            saved,
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
        own.chain(self.saved.all())
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
        own.chain(self.saved.all_of(namespace))
            .flat_map(|entries| entries.visible(scopes, phase))
            .map(|entry| entry.binding())
            .collect()
    }

    pub fn qualified_bindings_ending_in(
        &self,
        last_segment: &Symbol,
        namespace: Namespace,
        phase: Phase,
    ) -> Vec<BindingId> {
        let mut visible = Vec::new();
        for name in self
            .qualified_names
            .ending_in(last_segment.shared_name(), namespace)
        {
            for entries in self.entries_named(last_segment.scopes(), Rc::clone(name), namespace) {
                entries.visit_visible(last_segment.scopes(), phase, |entry| {
                    visible.push(entry.binding());
                });
            }
        }
        visible
    }

    #[inline(always)]
    pub(super) fn entries_named(
        &self,
        scopes: &ScopeSet,
        name: Rc<str>,
        namespace: Namespace,
    ) -> impl Iterator<Item = &'t NameEntries> + use<'t> {
        let saved = self.saved.entries_seen_from(scopes, &name, namespace);
        let own = self.entries.get(&(name, namespace));
        own.into_iter().chain(saved)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::super::binding_table::tests::World;
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
                .qualified_bindings_ending_in(
                    &World::symbol(last_segment, scopes),
                    Namespace::Value,
                    phase,
                );
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
}
