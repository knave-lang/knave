use rustc_hash::{FxHashMap, FxHashSet};

use super::connected_flows::ConnectedFlows;
use super::fact_rows::FactRows;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_flow::RecordedFlow;
use super::recorded_store::RecordedStore;
use crate::core::BindingId;

pub struct RegionFlow<H> {
    reaching: ReachingFacts,
    regions: RegionSets,
    stores: Vec<RegionStore<H>>,
}

struct RegionStore<H> {
    regions: FxHashSet<usize>,
    held: H,
}

impl<H> RegionFlow<H> {
    pub fn of(graph: &PointGraph, flows: Vec<RecordedFlow>, stores: Vec<RecordedStore<H>>) -> Self {
        let targets: Vec<BindingId> = stores.iter().map(|store| store.target).collect();
        let store_points: Vec<PointId> = stores.iter().map(|store| store.point).collect();
        let reaching = if stores.is_empty() {
            ReachingFacts::default()
        } else {
            ReachingFacts::of(graph, ConnectedFlows::kept(flows, &targets), &store_points)
        };
        let mut regions = RegionSets::new(reaching.flows.len());
        reaching.unify_sources(&mut regions);
        let stores = stores
            .into_iter()
            .map(|store| RegionStore {
                regions: reaching.regions_of(store.target, store.point, &mut regions),
                held: store.held,
            })
            .collect();
        Self {
            reaching,
            regions,
            stores,
        }
    }

    pub fn held_at(&self, point: PointId, binding: BindingId) -> Vec<&H> {
        if self.stores.is_empty() {
            return Vec::new();
        }
        let regions = self
            .reaching
            .known_regions_of(binding, point, &self.regions);
        self.stores
            .iter()
            .enumerate()
            .filter(|(index, _)| self.reaching.store_reaches(*index, point))
            .filter(|(_, store)| {
                store
                    .regions
                    .iter()
                    .any(|region| regions.contains(&self.regions.root(*region)))
            })
            .map(|(_, store)| &store.held)
            .collect()
    }
}

struct ReachingFacts {
    flows: Vec<RecordedFlow>,
    by_binding: FxHashMap<BindingId, Vec<usize>>,
    rows: FactRows,
}

impl Default for ReachingFacts {
    fn default() -> Self {
        Self {
            flows: Vec::new(),
            by_binding: FxHashMap::default(),
            rows: FactRows::new(0, 0),
        }
    }
}

struct PointFactChanges {
    generated: FxHashMap<PointId, Vec<usize>>,
    killed: FxHashMap<PointId, Vec<usize>>,
}

impl ReachingFacts {
    fn of(graph: &PointGraph, flows: Vec<RecordedFlow>, store_points: &[PointId]) -> Self {
        let mut by_binding: FxHashMap<BindingId, Vec<usize>> = FxHashMap::default();
        for (index, flow) in flows.iter().enumerate() {
            by_binding.entry(flow.binding).or_default().push(index);
        }
        let rows = graph
            .points()
            .map(|(point, _)| point.index() + 1)
            .max()
            .unwrap_or_default();
        let mut reaching = Self {
            rows: FactRows::new(rows, flows.len() + store_points.len()),
            flows,
            by_binding,
        };
        let changes = reaching.changes(store_points);
        let mut leaving = Vec::new();
        while reaching.grow(graph, &changes, &mut leaving) {}
        reaching
    }

    fn changes(&self, store_points: &[PointId]) -> PointFactChanges {
        let mut generated: FxHashMap<PointId, Vec<usize>> = FxHashMap::default();
        for (index, flow) in self.flows.iter().enumerate() {
            generated.entry(flow.point).or_default().push(index);
        }
        let mut killed: FxHashMap<PointId, Vec<usize>> = FxHashMap::default();
        for (point, defined) in &generated {
            let bindings = defined.iter().map(|flow| self.flows[*flow].binding);
            let same_binding =
                bindings.flat_map(|binding| self.by_binding[&binding].iter().copied());
            killed.insert(
                *point,
                same_binding
                    .filter(|flow| !defined.contains(flow))
                    .collect(),
            );
        }
        for (index, point) in store_points.iter().enumerate() {
            generated
                .entry(*point)
                .or_default()
                .push(self.flows.len() + index);
        }
        PointFactChanges { generated, killed }
    }

    fn grow(
        &mut self,
        graph: &PointGraph,
        changes: &PointFactChanges,
        leaving: &mut Vec<u64>,
    ) -> bool {
        let mut grew = false;
        for (point, facts) in graph.points() {
            self.rows.copy_row(point.index(), leaving);
            for flow in changes.killed.get(&point).into_iter().flatten() {
                FactRows::clear(leaving, *flow);
            }
            for fact in changes.generated.get(&point).into_iter().flatten() {
                FactRows::set(leaving, *fact);
            }
            for successor in facts.successors() {
                grew |= self.rows.absorb(successor.index(), leaving);
            }
        }
        grew
    }

    fn store_reaches(&self, store: usize, point: PointId) -> bool {
        self.rows.contains(point.index(), self.flows.len() + store)
    }

    fn unify_sources(&self, regions: &mut RegionSets) {
        for (index, flow) in self.flows.iter().enumerate() {
            for source in &flow.sources {
                for region in self.regions_of(*source, flow.point, regions) {
                    regions.unite(index, region);
                }
            }
        }
    }

    fn regions_of(
        &self,
        binding: BindingId,
        point: PointId,
        regions: &mut RegionSets,
    ) -> FxHashSet<usize> {
        let reaching = self.reaching_flows(binding, point);
        if reaching.is_empty() {
            return [regions.entry_of(binding)].into_iter().collect();
        }
        reaching.into_iter().collect()
    }

    fn known_regions_of(
        &self,
        binding: BindingId,
        point: PointId,
        regions: &RegionSets,
    ) -> FxHashSet<usize> {
        let reaching = self.reaching_flows(binding, point);
        if reaching.is_empty() {
            return regions
                .known_entry_of(binding)
                .map(|entry| regions.root(entry))
                .into_iter()
                .collect();
        }
        reaching
            .into_iter()
            .map(|flow| regions.root(flow))
            .collect()
    }

    fn reaching_flows(&self, binding: BindingId, point: PointId) -> Vec<usize> {
        self.by_binding
            .get(&binding)
            .into_iter()
            .flatten()
            .filter(|flow| self.rows.contains(point.index(), **flow))
            .copied()
            .collect()
    }
}

struct RegionSets {
    parents: Vec<usize>,
    entries: FxHashMap<BindingId, usize>,
}

impl RegionSets {
    fn new(flows: usize) -> Self {
        Self {
            parents: (0..flows).collect(),
            entries: FxHashMap::default(),
        }
    }

    fn entry_of(&mut self, binding: BindingId) -> usize {
        let next = self.parents.len();
        let entry = *self.entries.entry(binding).or_insert(next);
        if entry == next {
            self.parents.push(next);
        }
        entry
    }

    fn known_entry_of(&self, binding: BindingId) -> Option<usize> {
        self.entries.get(&binding).copied()
    }

    fn unite(&mut self, first: usize, second: usize) {
        let (first, second) = (self.root(first), self.root(second));
        self.parents[first] = second;
    }

    fn root(&self, region: usize) -> usize {
        let mut region = region;
        while self.parents[region] != region {
            region = self.parents[region];
        }
        region
    }
}

#[cfg(test)]
mod tests {
    use super::RegionFlow;
    use crate::core::BindingId;
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::point_id::PointId;
    use crate::ownership::loans::recorded_flow::RecordedFlow;
    use crate::ownership::loans::recorded_store::RecordedStore;

    const NS: BindingId = BindingId::from_index(1);
    const MS: BindingId = BindingId::from_index(2);
    const FIRST: BindingId = BindingId::from_index(3);

    #[derive(Default)]
    struct FlowFixture {
        graph: ControlFlowGraph,
        flows: Vec<RecordedFlow>,
        stores: Vec<RecordedStore<usize>>,
    }

    impl FlowFixture {
        fn after(&mut self, point: PointId) -> PointId {
            self.graph.join(&[point])
        }

        fn define(&mut self, point: PointId, binding: BindingId, sources: &[BindingId]) {
            self.graph.define(point, binding);
            self.flows.push(RecordedFlow {
                point,
                binding,
                sources: sources.to_vec(),
            });
        }

        fn store(&mut self, point: PointId, target: BindingId) {
            let held = self.stores.len();
            self.stores.push(RecordedStore {
                point,
                target,
                held,
            });
        }

        fn defined_after(
            &mut self,
            point: PointId,
            binding: BindingId,
            sources: &[BindingId],
        ) -> PointId {
            let defined = self.after(point);
            self.define(defined, binding, sources);
            defined
        }

        fn stored_after(&mut self, point: PointId, target: BindingId) -> PointId {
            let stored = self.after(point);
            self.store(stored, target);
            stored
        }

        fn flow(self) -> RegionFlow<usize> {
            RegionFlow::of(&self.graph.finished(), self.flows, self.stores)
        }
    }

    fn held(flow: &RegionFlow<usize>, point: PointId, binding: BindingId) -> Vec<usize> {
        let mut held: Vec<usize> = flow.held_at(point, binding).into_iter().copied().collect();
        held.sort_unstable();
        held
    }

    #[test]
    fn a_value_read_from_the_target_after_the_store_holds_it() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let read = fixture.defined_after(stored, FIRST, &[NS]);
        let later = fixture.after(read);
        assert_eq!(held(&fixture.flow(), later, FIRST), [0]);
    }

    #[test]
    fn a_value_read_from_the_target_before_the_store_shares_its_region() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let read = fixture.defined_after(bound, FIRST, &[NS]);
        let stored = fixture.stored_after(read, NS);
        let later = fixture.after(stored);
        let flow = fixture.flow();
        assert!(held(&flow, stored, FIRST).is_empty());
        assert_eq!(held(&flow, later, FIRST), [0]);
    }

    #[test]
    fn a_target_rebuilt_from_itself_keeps_its_stores() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let rebuilt = fixture.defined_after(stored, NS, &[NS]);
        let later = fixture.after(rebuilt);
        assert_eq!(held(&fixture.flow(), later, NS), [0]);
    }

    #[test]
    fn a_loop_binder_recurred_with_the_target_keeps_its_stores() {
        let mut fixture = FlowFixture::default();
        let entered = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let head = fixture.after(entered);
        let stored = fixture.stored_after(head, NS);
        let recurred = fixture.defined_after(stored, NS, &[NS]);
        fixture.graph.link(recurred, head);
        assert_eq!(held(&fixture.flow(), head, NS), [0]);
    }

    #[test]
    fn a_target_built_from_another_target_takes_its_stores() {
        let mut fixture = FlowFixture::default();
        let other = fixture.defined_after(PointId::ENTRY, MS, &[]);
        let stored = fixture.stored_after(other, MS);
        let bound = fixture.defined_after(stored, NS, &[MS]);
        let again = fixture.stored_after(bound, NS);
        let later = fixture.after(again);
        assert_eq!(held(&fixture.flow(), later, NS), [0, 1]);
    }

    #[test]
    fn a_loop_binder_entered_from_another_target_takes_its_stores() {
        let mut fixture = FlowFixture::default();
        let other = fixture.defined_after(PointId::ENTRY, MS, &[]);
        let stored = fixture.stored_after(other, MS);
        let entered = fixture.defined_after(stored, NS, &[MS]);
        let head = fixture.after(entered);
        let recurred = fixture.defined_after(head, NS, &[NS]);
        fixture.graph.link(recurred, head);
        assert_eq!(held(&fixture.flow(), head, NS), [0]);
    }

    #[test]
    fn a_target_moved_away_leaves_its_stores_with_the_new_binding() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let moved = fixture.defined_after(stored, MS, &[NS]);
        let later = fixture.after(moved);
        assert_eq!(held(&fixture.flow(), later, MS), [0]);
    }

    #[test]
    fn a_reset_to_a_fresh_value_drops_the_earlier_stores() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let reset = fixture.defined_after(stored, NS, &[]);
        let read = fixture.defined_after(reset, FIRST, &[NS]);
        let later = fixture.after(read);
        let flow = fixture.flow();
        assert!(held(&flow, later, NS).is_empty());
        assert!(held(&flow, later, FIRST).is_empty());
    }

    #[test]
    fn a_store_after_a_reset_is_held() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let reset = fixture.defined_after(bound, NS, &[]);
        let stored = fixture.stored_after(reset, NS);
        let later = fixture.after(stored);
        assert_eq!(held(&fixture.flow(), later, NS), [0]);
    }

    #[test]
    fn a_change_before_the_store_is_not_reached_by_it() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let changed = fixture.after(bound);
        let stored = fixture.stored_after(changed, NS);
        let later = fixture.after(stored);
        let flow = fixture.flow();
        assert!(held(&flow, changed, NS).is_empty());
        assert_eq!(held(&flow, later, NS), [0]);
    }

    #[test]
    fn a_reset_on_one_arm_keeps_a_store_made_before_the_branch() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let reset = fixture.defined_after(stored, NS, &[]);
        let kept = fixture.after(stored);
        let joined = fixture.graph.join(&[reset, kept]);
        assert_eq!(held(&fixture.flow(), joined, NS), [0]);
    }

    #[test]
    fn a_reset_on_one_arm_keeps_the_store_of_the_other_arm_and_a_later_one() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let reset = fixture.defined_after(bound, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let joined = fixture.graph.join(&[reset, stored]);
        let again = fixture.stored_after(joined, NS);
        let later = fixture.after(again);
        let flow = fixture.flow();
        assert_eq!(held(&flow, joined, NS), [0]);
        assert_eq!(held(&flow, later, NS), [0, 1]);
    }

    #[test]
    fn a_store_in_an_inner_loop_reaches_the_heads_of_both_loops() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let outer = fixture.after(bound);
        let inner = fixture.after(outer);
        let stored = fixture.stored_after(inner, NS);
        fixture.graph.link(stored, inner);
        let left = fixture.after(inner);
        fixture.graph.link(left, outer);
        let flow = fixture.flow();
        assert!(held(&flow, bound, NS).is_empty());
        assert_eq!(held(&flow, inner, NS), [0]);
        assert_eq!(held(&flow, outer, NS), [0]);
    }

    #[test]
    fn a_binding_apart_from_every_target_holds_no_store() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let apart = fixture.defined_after(bound, FIRST, &[]);
        let copied = fixture.defined_after(apart, MS, &[FIRST]);
        let stored = fixture.stored_after(copied, NS);
        let later = fixture.after(stored);
        let flow = fixture.flow();
        assert!(held(&flow, later, MS).is_empty());
        assert!(held(&flow, later, FIRST).is_empty());
        assert_eq!(held(&flow, later, NS), [0]);
    }

    #[test]
    fn many_stores_in_one_body_each_last_until_the_next_reset() {
        const STORES: usize = 400;
        let mut fixture = FlowFixture::default();
        let mut point = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let mut between = Vec::new();
        for _ in 0..STORES {
            let stored = fixture.stored_after(point, NS);
            let read = fixture.after(stored);
            between.push(read);
            point = fixture.defined_after(read, NS, &[]);
        }
        let end = fixture.after(point);
        let flow = fixture.flow();
        for (store, read) in between.into_iter().enumerate() {
            assert_eq!(held(&flow, read, NS), [store]);
        }
        assert!(held(&flow, end, NS).is_empty());
    }
}
