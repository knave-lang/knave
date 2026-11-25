use std::collections::hash_map::Entry;

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
    regions: FxHashSet<Region>,
    held: H,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FlowId(usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Region(usize);

impl FlowId {
    fn fact(self) -> usize {
        self.0
    }
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
        let regions = reaching.unified_regions(&targets);
        let stores = stores
            .into_iter()
            .map(|store| RegionStore {
                regions: reaching.regions_of(store.target, store.point, &regions),
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
        let regions = self.reaching.regions_of(binding, point, &self.regions);
        self.stores
            .iter()
            .enumerate()
            .filter(|(index, _)| self.reaching.store_reaches(*index, point))
            .filter(|(_, store)| !store.regions.is_disjoint(&regions))
            .map(|(_, store)| &store.held)
            .collect()
    }
}

struct ReachingFacts {
    flows: Vec<RecordedFlow>,
    by_binding: FxHashMap<BindingId, Vec<FlowId>>,
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
        let mut by_binding: FxHashMap<BindingId, Vec<FlowId>> = FxHashMap::default();
        for (index, flow) in flows.iter().enumerate() {
            by_binding
                .entry(flow.binding)
                .or_default()
                .push(FlowId(index));
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
        let mut defined: FxHashMap<PointId, Vec<FlowId>> = FxHashMap::default();
        for (flow, recorded) in self.recorded() {
            defined.entry(recorded.point).or_default().push(flow);
        }
        let killed = defined
            .iter()
            .map(|(point, flows)| (*point, self.killed_by(flows)))
            .collect();
        let mut generated: FxHashMap<PointId, Vec<usize>> = defined
            .into_iter()
            .map(|(point, flows)| (point, flows.into_iter().map(FlowId::fact).collect()))
            .collect();
        for (index, point) in store_points.iter().enumerate() {
            generated
                .entry(*point)
                .or_default()
                .push(self.store_fact(index));
        }
        PointFactChanges { generated, killed }
    }

    fn killed_by(&self, defined: &[FlowId]) -> Vec<usize> {
        defined
            .iter()
            .flat_map(|flow| &self.by_binding[&self.flows[flow.0].binding])
            .filter(|other| !defined.contains(*other))
            .map(|other| other.fact())
            .collect()
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
            for fact in changes.killed.get(&point).into_iter().flatten() {
                FactRows::clear(leaving, *fact);
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
        self.rows.contains(point.index(), self.store_fact(store))
    }

    fn store_fact(&self, store: usize) -> usize {
        self.flows.len() + store
    }

    fn unified_regions(&self, targets: &[BindingId]) -> RegionSets {
        let sources = self.flows.iter().flat_map(RecordedFlow::sources);
        let mut regions =
            RegionSets::starting_with(self.flows.len(), sources.chain(targets.iter().copied()));
        for (flow, recorded) in self.recorded() {
            let joined: Vec<Region> = recorded
                .sources()
                .flat_map(|source| self.regions_of(source, recorded.point, &regions))
                .collect();
            for region in joined {
                regions.unite(regions.of_flow(flow), region);
            }
        }
        regions
    }

    fn regions_of(
        &self,
        binding: BindingId,
        point: PointId,
        regions: &RegionSets,
    ) -> FxHashSet<Region> {
        let reaching = self.reaching_flows(binding, point);
        if reaching.is_empty() {
            return regions.entry_of(binding).into_iter().collect();
        }
        reaching
            .into_iter()
            .map(|flow| regions.of_flow(flow))
            .collect()
    }

    fn reaching_flows(&self, binding: BindingId, point: PointId) -> Vec<FlowId> {
        self.by_binding
            .get(&binding)
            .into_iter()
            .flatten()
            .filter(|flow| self.rows.contains(point.index(), flow.fact()))
            .copied()
            .collect()
    }

    fn recorded(&self) -> impl Iterator<Item = (FlowId, &RecordedFlow)> {
        self.flows
            .iter()
            .enumerate()
            .map(|(index, recorded)| (FlowId(index), recorded))
    }
}

struct RegionSets {
    parents: Vec<Region>,
    entries: FxHashMap<BindingId, Region>,
}

impl RegionSets {
    fn starting_with(flows: usize, entered: impl Iterator<Item = BindingId>) -> Self {
        let mut regions = Self {
            parents: (0..flows).map(Region).collect(),
            entries: FxHashMap::default(),
        };
        for binding in entered {
            regions.enter(binding);
        }
        regions
    }

    fn enter(&mut self, binding: BindingId) {
        let next = Region(self.parents.len());
        if let Entry::Vacant(vacant) = self.entries.entry(binding) {
            vacant.insert(next);
            self.parents.push(next);
        }
    }

    fn of_flow(&self, flow: FlowId) -> Region {
        self.root(Region(flow.0))
    }

    fn entry_of(&self, binding: BindingId) -> Option<Region> {
        self.entries.get(&binding).map(|entry| self.root(*entry))
    }

    fn unite(&mut self, first: Region, second: Region) {
        let (first, second) = (self.root(first), self.root(second));
        self.parents[first.0] = second;
    }

    fn root(&self, region: Region) -> Region {
        let mut region = region;
        while self.parents[region.0] != region {
            region = self.parents[region.0];
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
            self.flows
                .push(RecordedFlow::from_sources(point, binding, sources));
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
