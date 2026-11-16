use std::collections::hash_map;
use std::ops::Range;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::Scope;
use super::name_entries::NameEntries;
use super::name_entry::NameEntry;
use super::phase::Phase;
use super::qualified_names::QualifiedNames;
use super::saved_names::{NameTables, SavedNames};
use super::visible_names::VisibleNames;
use crate::core::{BindingId, Namespace};
use crate::reader::Symbol;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Bound(BindingId),
    Unbound,
    Ambiguous(Vec<BindingId>),
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct BindingTable {
    #[serde(skip)]
    saved: SavedNames,
    entries: FxHashMap<(Rc<str>, Namespace), NameEntries>,
    qualified_names: QualifiedNames,
}

impl BindingTable {
    pub fn bind(
        &mut self,
        symbol: &Symbol,
        namespace: Namespace,
        binding: BindingId,
        phase: Phase,
    ) -> Result<(), BindingId> {
        let entries = match self.entries.entry((symbol.shared_name(), namespace)) {
            hash_map::Entry::Occupied(occupied) => occupied.into_mut(),
            hash_map::Entry::Vacant(vacant) => {
                self.qualified_names.record(vacant.key());
                vacant.insert(NameEntries::default())
            }
        };
        entries.bind(symbol.scopes(), binding, phase)
    }

    pub fn unbind(&mut self, symbol: &Symbol, namespace: Namespace, binding: BindingId) {
        if let Some(entries) = self.entries.get_mut(&(symbol.shared_name(), namespace)) {
            entries.unbind(binding);
        }
    }

    pub fn forget(&mut self, forgotten: impl Fn(BindingId) -> bool) {
        for entries in self.entries.values_mut() {
            entries.forget(&forgotten);
        }
        self.entries.retain(|_, entries| !entries.is_empty());
    }

    #[cfg(test)]
    pub fn holds(&self, binding: BindingId) -> bool {
        let saved = self.saved.all().map(|(_, _, entries)| entries);
        let mut all = self.entries.values().chain(saved);
        all.any(|entries| entries.holds(binding))
    }

    pub fn resolve(&self, symbol: &Symbol, namespace: Namespace, phase: Phase) -> Resolution {
        let mut largest = LargestCandidates::default();
        for entries in
            self.visible_names()
                .entries_named(symbol.scopes(), symbol.shared_name(), namespace)
        {
            entries.visit_visible(symbol.scopes(), phase, |entry| largest.consider(entry));
        }
        largest.resolution()
    }

    #[inline(always)]
    pub fn visible_names(&self) -> VisibleNames<'_> {
        VisibleNames::new(&self.entries, &self.saved, &self.qualified_names)
    }

    pub fn take_saved(&mut self, prelude_scopes: Range<Scope>) -> NameTables {
        let entries = std::mem::take(&mut self.entries);
        let keyed = entries
            .into_iter()
            .map(|((name, namespace), entries)| (name, namespace, entries));
        NameTables::of(keyed.collect(), prelude_scopes)
    }

    pub fn adopt_saved(&mut self, saved: SavedNames) {
        self.saved = saved;
    }
}

#[derive(Default)]
struct LargestCandidates {
    size: usize,
    first: Option<BindingId>,
    others: Vec<BindingId>,
}

impl LargestCandidates {
    fn consider(&mut self, entry: &NameEntry) {
        let size = entry.scopes().len();
        match self.first {
            Some(_) if size < self.size => {}
            Some(first) if size == self.size => self.add_equal(first, entry.binding()),
            _ => self.replace(size, entry.binding()),
        }
    }

    fn add_equal(&mut self, first: BindingId, binding: BindingId) {
        if binding != first {
            self.others.push(binding);
        }
    }

    fn replace(&mut self, size: usize, binding: BindingId) {
        self.size = size;
        self.first = Some(binding);
        self.others.clear();
    }

    fn resolution(self) -> Resolution {
        match self.first {
            None => Resolution::Unbound,
            Some(binding) if self.others.is_empty() => Resolution::Bound(binding),
            Some(binding) => Resolution::Ambiguous(Self::distinct(binding, self.others)),
        }
    }

    fn distinct(first: BindingId, mut others: Vec<BindingId>) -> Vec<BindingId> {
        others.push(first);
        others.sort();
        others.dedup();
        others
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::core::{BindingArena, BindingInfo, BindingKind};
    use crate::scope::{ChainPrefix, Scope, ScopeChain, ScopeKind, ScopeSet, Scopes};

    pub(in crate::scope) struct World {
        pub(in crate::scope) table: BindingTable,
        bindings: BindingArena,
        scopes: Scopes,
    }

    impl World {
        const LONG_LIST_LENGTH: usize = 20;

        pub(in crate::scope) fn new() -> Self {
            Self {
                table: BindingTable::default(),
                bindings: BindingArena::default(),
                scopes: Scopes::default(),
            }
        }

        pub(in crate::scope) fn scope(&mut self) -> Scope {
            self.scopes.fresh(ScopeKind::Local)
        }

        pub(in crate::scope) fn save_names(&mut self) {
            let first = Scopes::default().next_scope();
            self.save_names_from(first);
        }

        fn save_names_from(&mut self, first: Scope) {
            let prelude_scopes = first..self.scopes.next_scope();
            let tables = self.table.take_saved(prelude_scopes);
            let encoded = bincode::serialize(&tables).expect("the tables serialize");
            let saved = bincode::deserialize(encoded.leak()).expect("the tables load");
            self.table.adopt_saved(saved);
        }

        fn chain(&mut self, length: usize) -> ScopeChain {
            let chain = ScopeChain::default();
            (0..length).for_each(|_| chain.push(self.scope()));
            chain
        }

        fn prefix(chain: &ScopeChain) -> ChainPrefix {
            chain.prefix().expect("a test chain holds scopes")
        }

        fn sharing_symbol(name: &str, prefix: &ChainPrefix) -> Symbol {
            let mut scopes = ScopeSet::default();
            scopes.add_prefix(prefix);
            Symbol::with_scopes(name, scopes)
        }

        fn define_sharing(&mut self, name: &str, prefix: &ChainPrefix) -> BindingId {
            let binding = self.new_binding(name);
            self.table
                .bind(
                    &Self::sharing_symbol(name, prefix),
                    Namespace::Value,
                    binding,
                    Phase::Runtime,
                )
                .expect("no duplicate");
            binding
        }

        fn new_binding(&mut self, name: &str) -> BindingId {
            let info = BindingInfo::new(name.into(), BindingKind::Local, None);
            self.bindings.add(info)
        }

        fn resolve_sharing(&self, name: &str, prefix: &ChainPrefix) -> Resolution {
            self.table.resolve(
                &Self::sharing_symbol(name, prefix),
                Namespace::Value,
                Phase::Runtime,
            )
        }

        pub(in crate::scope) fn symbol(name: &str, scopes: &[Scope]) -> Symbol {
            Symbol::with_scopes(name, Self::scope_set(scopes))
        }

        pub(in crate::scope) fn scope_set(scopes: &[Scope]) -> ScopeSet {
            let mut set = ScopeSet::default();
            scopes.iter().for_each(|scope| set.add(*scope));
            set
        }

        pub(in crate::scope) fn define(
            &mut self,
            name: &str,
            scopes: &[Scope],
            phase: Phase,
        ) -> BindingId {
            self.define_in(name, scopes, Namespace::Value, phase)
        }

        pub(in crate::scope) fn define_type(&mut self, name: &str, scopes: &[Scope]) -> BindingId {
            self.define_in(name, scopes, Namespace::Type, Phase::Runtime)
        }

        fn define_in(
            &mut self,
            name: &str,
            scopes: &[Scope],
            namespace: Namespace,
            phase: Phase,
        ) -> BindingId {
            let binding = self.new_binding(name);
            let symbol = Self::symbol(name, scopes);
            self.table
                .bind(&symbol, namespace, binding, phase)
                .expect("no duplicate");
            binding
        }

        fn define_under_subsets(
            &mut self,
            name: &str,
            scopes: &[Scope],
            count: usize,
        ) -> Vec<(Vec<Scope>, BindingId)> {
            (0..count)
                .map(|index| {
                    let subset = Self::subset(scopes, index);
                    let binding = self.define(name, &subset, Phase::Runtime);
                    (subset, binding)
                })
                .collect()
        }

        fn subset(scopes: &[Scope], index: usize) -> Vec<Scope> {
            scopes
                .iter()
                .enumerate()
                .filter(|(position, _)| index & (1 << position) != 0)
                .map(|(_, scope)| *scope)
                .collect()
        }

        fn resolve(&self, name: &str, scopes: &[Scope], phase: Phase) -> Resolution {
            self.table
                .resolve(&Self::symbol(name, scopes), Namespace::Value, phase)
        }
    }

    #[test]
    fn taking_the_saved_names_leaves_the_table_empty() {
        let mut world = World::new();
        let scope = world.scope();
        world.define("taken", &[scope], Phase::Runtime);
        let everything = Scopes::default().next_scope()..world.scopes.next_scope();
        world.table.take_saved(everything);
        assert!(world.table.entries.is_empty());
    }

    #[test]
    fn a_symbol_with_a_prelude_scope_finds_a_saved_name() {
        let mut world = World::new();
        let scope = world.scope();
        let saved = world.define("saved", &[scope], Phase::Runtime);
        world.save_names();
        let symbol = World::symbol("saved", &[scope]);
        let resolution = world
            .table
            .resolve(&symbol, Namespace::Value, Phase::Runtime);
        assert_eq!(resolution, Resolution::Bound(saved));
    }

    #[test]
    fn a_symbol_with_only_later_scopes_decodes_no_saved_name() {
        let mut world = World::new();
        let scope = world.scope();
        world.define("saved", &[scope], Phase::Runtime);
        world.save_names();
        let later = world.scope();
        let symbol = World::symbol("saved", &[later]);
        world
            .table
            .resolve(&symbol, Namespace::Value, Phase::Runtime);
        assert!(!world.table.saved.is_decoded("saved", Namespace::Value));
    }

    #[test]
    fn a_symbol_with_only_earlier_scopes_decodes_no_saved_name() {
        let mut world = World::new();
        let earlier = world.scope();
        let first = world.scopes.next_scope();
        let scope = world.scope();
        world.define("saved", &[scope], Phase::Runtime);
        world.save_names_from(first);
        let symbol = World::symbol("saved", &[earlier]);
        world
            .table
            .resolve(&symbol, Namespace::Value, Phase::Runtime);
        assert!(!world.table.saved.is_decoded("saved", Namespace::Value));
    }

    #[test]
    fn a_shared_scope_chain_still_looks_in_the_saved_names() {
        let mut world = World::new();
        let scope = world.scope();
        world.define("saved", &[scope], Phase::Runtime);
        world.save_names();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        world.resolve_sharing("saved", &World::prefix(&chain));
        assert!(world.table.saved.is_decoded("saved", Namespace::Value));
    }

    #[test]
    #[should_panic(expected = "every saved name is bound under a prelude scope")]
    fn a_saved_name_outside_the_prelude_scopes_stops_the_build() {
        let mut world = World::new();
        let scope = world.scope();
        world.define("saved", &[scope], Phase::Runtime);
        let later = world.scopes.next_scope();
        world.table.take_saved(later..later);
    }

    #[test]
    fn a_qualified_name_bound_again_after_a_save_is_recorded_once() {
        let mut world = World::new();
        let (scope, other) = (world.scope(), world.scope());
        world.define("option/some", &[scope], Phase::Runtime);
        world.save_names();
        world.define("option/some", &[other], Phase::Runtime);
        let recorded = world
            .table
            .qualified_names
            .ending_in("some".into(), Namespace::Value);
        assert_eq!(recorded.len(), 1);
    }

    #[test]
    #[should_panic(expected = "every saved name is bound under a prelude scope")]
    fn a_shared_saved_name_outside_the_prelude_scopes_stops_the_build() {
        let mut world = World::new();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        world.define_sharing("saved", &World::prefix(&chain));
        let later = world.scopes.next_scope();
        world.table.take_saved(later..later);
    }

    #[test]
    fn a_program_name_beside_a_saved_one_still_sees_the_saved_one() {
        let mut world = World::new();
        let scope = world.scope();
        let saved = world.define("shared", &[scope], Phase::Runtime);
        world.save_names();
        let later = world.scope();
        world.define("shared", &[later], Phase::Runtime);
        let symbol = World::symbol("shared", &[scope]);
        let resolve = || {
            world
                .table
                .resolve(&symbol, Namespace::Value, Phase::Runtime)
        };
        assert_eq!(resolve(), Resolution::Bound(saved));
        assert_eq!(resolve(), Resolution::Bound(saved));
    }

    #[test]
    fn a_program_name_with_only_later_scopes_decodes_no_saved_name() {
        let mut world = World::new();
        let scope = world.scope();
        world.define("shared", &[scope], Phase::Runtime);
        world.save_names();
        let later = world.scope();
        world.define("shared", &[later], Phase::Runtime);
        let symbol = World::symbol("shared", &[later]);
        world
            .table
            .resolve(&symbol, Namespace::Value, Phase::Runtime);
        assert!(!world.table.saved.is_decoded("shared", Namespace::Value));
    }

    #[test]
    fn a_forgotten_binding_resolves_no_more() {
        let mut world = World::new();
        let scope = world.scope();
        let kept = world.define("kept", &[scope], Phase::Runtime);
        let forgotten = world.define("forgotten", &[scope], Phase::Runtime);
        world.table.forget(|binding| binding == forgotten);
        let resolve = |name| {
            world.table.resolve(
                &World::symbol(name, &[scope]),
                Namespace::Value,
                Phase::Runtime,
            )
        };
        assert_eq!(resolve("forgotten"), Resolution::Unbound);
        assert_eq!(resolve("kept"), Resolution::Bound(kept));
        assert!(!world.table.holds(forgotten));
        assert!(world.table.holds(kept));
    }

    #[test]
    fn a_forgotten_sharing_binding_resolves_no_more() {
        let mut world = World::new();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        let prefix = World::prefix(&chain);
        let forgotten = world.define_sharing("shared", &prefix);
        world.table.forget(|binding| binding == forgotten);
        assert_eq!(
            world.resolve_sharing("shared", &prefix),
            Resolution::Unbound
        );
    }

    #[test]
    fn forgetting_every_binding_of_a_name_drops_the_name() {
        let mut world = World::new();
        let scope = world.scope();
        let forgotten = world.define("gone", &[scope], Phase::Runtime);
        world.table.forget(|binding| binding == forgotten);
        assert!(world.table.entries.is_empty());
    }

    #[test]
    fn forgetting_one_of_two_bindings_of_a_name_keeps_the_other() {
        let mut world = World::new();
        let (outer, inner) = (world.scope(), world.scope());
        let wide = world.define("x", &[outer], Phase::Runtime);
        let narrow = world.define("x", &[outer, inner], Phase::Runtime);
        world.table.forget(|binding| binding == narrow);
        let symbol = World::symbol("x", &[outer, inner]);
        let resolution = world
            .table
            .resolve(&symbol, Namespace::Value, Phase::Runtime);
        assert_eq!(resolution, Resolution::Bound(wide));
    }

    #[test]
    fn the_binding_with_the_largest_subset_wins() {
        let mut world = World::new();
        let (module, inner) = (world.scope(), world.scope());
        world.define("x", &[module], Phase::Runtime);
        let shadow = world.define("x", &[module, inner], Phase::Runtime);
        assert_eq!(
            world.resolve("x", &[module, inner], Phase::Runtime),
            Resolution::Bound(shadow)
        );
    }

    #[test]
    fn a_binding_outside_the_reference_scopes_is_invisible() {
        let mut world = World::new();
        let (module, macro_introduced) = (world.scope(), world.scope());
        world.define("tmp", &[module, macro_introduced], Phase::Runtime);
        assert_eq!(
            world.resolve("tmp", &[module], Phase::Runtime),
            Resolution::Unbound
        );
    }

    #[test]
    fn two_bindings_of_equal_size_are_ambiguous() {
        let mut world = World::new();
        let (module, left, right) = (world.scope(), world.scope(), world.scope());
        world.define("x", &[module, left], Phase::Runtime);
        world.define("x", &[module, right], Phase::Runtime);
        let resolution = world.resolve("x", &[module, left, right], Phase::Runtime);
        assert!(matches!(resolution, Resolution::Ambiguous(_)));
    }

    #[test]
    fn a_larger_binding_settles_two_smaller_bindings_of_equal_size() {
        let mut world = World::new();
        let (module, left, right) = (world.scope(), world.scope(), world.scope());
        world.define("x", &[module, left], Phase::Runtime);
        world.define("x", &[module, right], Phase::Runtime);
        let settled = world.define("x", &[module, left, right], Phase::Runtime);
        assert_eq!(
            world.resolve("x", &[module, left, right], Phase::Runtime),
            Resolution::Bound(settled)
        );
    }

    #[test]
    fn compile_time_bindings_are_invisible_to_runtime_code() {
        let mut world = World::new();
        let module = world.scope();
        world.define("name", &[module], Phase::CompileTime);
        assert_eq!(
            world.resolve("name", &[module], Phase::Runtime),
            Resolution::Unbound
        );
    }

    #[test]
    fn rebinding_a_binding_to_its_own_scopes_is_accepted() {
        let mut world = World::new();
        let module = world.scope();
        let binding = world.define("x", &[module], Phase::Runtime);
        let rebound = world.table.bind(
            &World::symbol("x", &[module]),
            Namespace::Value,
            binding,
            Phase::Runtime,
        );
        assert_eq!(rebound, Ok(()));
    }

    #[test]
    fn rebinding_the_same_scopes_reports_the_existing_binding() {
        let mut world = World::new();
        let module = world.scope();
        let first = world.define("x", &[module], Phase::Runtime);
        let info = BindingInfo::new("x".into(), BindingKind::Local, None);
        let second = world.bindings.add(info);
        let duplicate = world.table.bind(
            &World::symbol("x", &[module]),
            Namespace::Value,
            second,
            Phase::Runtime,
        );
        assert_eq!(duplicate, Err(first));
    }

    #[test]
    fn a_long_binding_list_resolves_to_its_exact_subset() {
        let mut world = World::new();
        let scopes: Vec<Scope> = (0..5).map(|_| world.scope()).collect();
        let entries = world.define_under_subsets("x", &scopes, World::LONG_LIST_LENGTH);
        let reference = [scopes[0], scopes[1], scopes[4]];
        let (_, exact) = entries
            .iter()
            .find(|(subset, _)| *subset == reference)
            .expect("the reference scopes are a defined subset");
        assert_eq!(
            world.resolve("x", &reference, Phase::Runtime),
            Resolution::Bound(*exact)
        );
    }

    #[test]
    fn a_binding_under_a_shared_prefix_is_visible_from_a_longer_prefix() {
        let mut world = World::new();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        let bound = world.define_sharing("x", &World::prefix(&chain));
        chain.push(world.scope());
        assert_eq!(
            world.resolve_sharing("x", &World::prefix(&chain)),
            Resolution::Bound(bound)
        );
    }

    #[test]
    fn a_shared_binding_is_visible_from_plain_scopes_that_hold_its_prefix() {
        let mut world = World::new();
        let scopes: Vec<Scope> = (0..ScopeSet::SHORTEST_SHARED_PREFIX)
            .map(|_| world.scope())
            .collect();
        let chain = ScopeChain::default();
        scopes.iter().for_each(|scope| chain.push(*scope));
        let bound = world.define_sharing("x", &World::prefix(&chain));
        assert_eq!(
            world.resolve("x", &scopes, Phase::Runtime),
            Resolution::Bound(bound)
        );
    }

    #[test]
    fn a_compile_time_shared_binding_is_invisible_to_runtime_code() {
        let mut world = World::new();
        let prefix = World::prefix(&world.chain(ScopeSet::SHORTEST_SHARED_PREFIX));
        let binding = world.new_binding("x");
        world
            .table
            .bind(
                &World::sharing_symbol("x", &prefix),
                Namespace::Value,
                binding,
                Phase::CompileTime,
            )
            .expect("no duplicate");
        assert_eq!(world.resolve_sharing("x", &prefix), Resolution::Unbound);
    }

    #[test]
    fn a_plain_binding_beside_a_shared_one_sees_only_its_own_scopes() {
        let mut world = World::new();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        world.define_sharing("x", &World::prefix(&chain));
        let outside = world.scope();
        let plain = world.define("x", &[outside], Phase::Runtime);
        assert_eq!(
            world.resolve("x", &[outside], Phase::Runtime),
            Resolution::Bound(plain)
        );
    }

    #[test]
    fn a_shared_prefix_sees_the_plain_binding_it_shadows_only_when_larger() {
        let mut world = World::new();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        let first = World::prefix(&chain).first();
        world.define("x", &[first], Phase::Runtime);
        let shadow = world.define_sharing("x", &World::prefix(&chain));
        assert_eq!(
            world.resolve_sharing("x", &World::prefix(&chain)),
            Resolution::Bound(shadow)
        );
    }

    #[test]
    fn rebinding_shared_scopes_reports_the_existing_binding() {
        let mut world = World::new();
        let chain = world.chain(ScopeSet::SHORTEST_SHARED_PREFIX);
        let prefix = World::prefix(&chain);
        let first = world.define_sharing("x", &prefix);
        let second = world.new_binding("x");
        let duplicate = world.table.bind(
            &World::sharing_symbol("x", &prefix),
            Namespace::Value,
            second,
            Phase::Runtime,
        );
        assert_eq!(duplicate, Err(first));
    }
}
