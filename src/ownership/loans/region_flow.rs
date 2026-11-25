use std::collections::BTreeSet;

use rustc_hash::{FxHashMap, FxHashSet};

use super::connected_flows::ConnectedFlows;
use super::forward_facts::ForwardFacts;
use super::live_values::LiveValues;
use super::loan_id::LoanId;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::reaching_facts::ReachingFacts;
use super::recorded_flow::RecordedFlow;
use super::recorded_store::RecordedStore;
use super::region_graph::{Region, RegionGraph};
use super::store_holders::StoreHolders;
use super::store_scope::StoreScope;
use super::stored_loans::StoredLoans;
use crate::core::BindingId;

#[derive(Default)]
pub struct RegionFlow {
    regions: RegionGraph,
    stores: Vec<RegionStore>,
    scope: ForwardFacts,
}

struct RegionStore {
    reach: FxHashSet<Region>,
    loans: BTreeSet<LoanId>,
}

impl RegionFlow {
    pub fn of(
        graph: &PointGraph,
        flows: Vec<RecordedFlow>,
        stores: Vec<RecordedStore<StoredLoans>>,
    ) -> Self {
        if stores.is_empty() {
            return Self::default();
        }
        let flows = ConnectedFlows::kept(flows, &stores);
        let regions = RegionGraph::linked(ReachingFacts::of(graph, flows), &stores);
        let (points, stores) = Self::reached(&regions, stores);
        let live = LiveValues::holding(graph, regions.flowing_at());
        let reaches = stores.iter().map(|store| &store.reach);
        let holders = StoreHolders::new(&regions, &live, reaches);
        let scope = StoreScope::of(graph, &holders, &points);
        let scope = ForwardFacts::solved(graph, stores.len(), &scope);
        Self {
            regions,
            stores,
            scope,
        }
    }

    pub fn held_at(&self, point: PointId, binding: BindingId) -> Vec<&BTreeSet<LoanId>> {
        let regions: FxHashSet<Region> = self
            .regions
            .regions_of(binding, point)
            .into_iter()
            .collect();
        self.stores
            .iter()
            .enumerate()
            .filter(|(store, _)| self.scope.entering(point, *store))
            .filter(|(_, store)| !store.reach.is_disjoint(&regions))
            .map(|(_, store)| &store.loans)
            .collect()
    }

    fn reached(
        regions: &RegionGraph,
        stores: Vec<RecordedStore<StoredLoans>>,
    ) -> (Vec<PointId>, Vec<RegionStore>) {
        let mut reaches: FxHashMap<Vec<Region>, FxHashSet<Region>> = FxHashMap::default();
        stores
            .into_iter()
            .map(|store| {
                let mut starts = regions.regions_of(store.target, store.point);
                starts.sort_unstable();
                let reach = reaches
                    .entry(starts)
                    .or_insert_with_key(|starts| regions.reached_from(starts))
                    .clone();
                let loans = store.held.loans;
                (store.point, RegionStore { reach, loans })
            })
            .unzip()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::RegionFlow;
    use crate::core::BindingId;
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::loan_id::LoanId;
    use crate::ownership::loans::point_id::PointId;
    use crate::ownership::loans::recorded_flow::RecordedFlow;
    use crate::ownership::loans::recorded_store::RecordedStore;
    use crate::ownership::loans::stored_loans::StoredLoans;
    use crate::source::{SourceId, Span};

    const NS: BindingId = BindingId::from_index(1);
    const MS: BindingId = BindingId::from_index(2);
    const FIRST: BindingId = BindingId::from_index(3);

    #[derive(Default)]
    struct FlowFixture {
        graph: ControlFlowGraph,
        flows: Vec<RecordedFlow>,
        stores: Vec<RecordedStore<StoredLoans>>,
    }

    impl FlowFixture {
        fn after(&mut self, point: PointId) -> PointId {
            self.graph.join(&[point])
        }

        fn defined_after(
            &mut self,
            point: PointId,
            binding: BindingId,
            sources: &[BindingId],
        ) -> PointId {
            let defined = self.after(point);
            self.graph.define(defined, binding);
            self.flows
                .push(RecordedFlow::from_sources(defined, binding, sources));
            defined
        }

        fn stored_after(&mut self, point: PointId, target: BindingId) -> PointId {
            self.stored_from(point, target, &[])
        }

        fn stored_from(
            &mut self,
            point: PointId,
            target: BindingId,
            sources: &[BindingId],
        ) -> PointId {
            let stored = self.after(point);
            let loans = BTreeSet::from([LoanId::new(self.stores.len())]);
            self.stores.push(RecordedStore {
                point: stored,
                target,
                held: StoredLoans {
                    sources: sources.to_vec(),
                    loans,
                },
            });
            stored
        }

        fn read_after(&mut self, point: PointId, binding: BindingId) -> PointId {
            let read = self.after(point);
            self.graph
                .using(read, binding, Span::new(SourceId::new(0), 0..1));
            read
        }

        fn flow(self) -> RegionFlow {
            RegionFlow::of(&self.graph.finished(), self.flows, self.stores)
        }

        fn held(flow: &RegionFlow, point: PointId, binding: BindingId) -> Vec<usize> {
            let mut held: Vec<usize> = flow
                .held_at(point, binding)
                .into_iter()
                .flatten()
                .map(|loan| loan.index())
                .collect();
            held.sort_unstable();
            held
        }
    }

    #[test]
    fn a_value_read_from_the_target_after_the_store_holds_it() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let read = fixture.defined_after(stored, FIRST, &[NS]);
        let later = fixture.read_after(read, FIRST);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, FIRST), [0]);
    }

    #[test]
    fn a_value_read_from_the_target_before_the_store_takes_later_stores() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let read = fixture.defined_after(bound, FIRST, &[NS]);
        let stored = fixture.stored_after(read, NS);
        let later = fixture.read_after(stored, FIRST);
        let flow = fixture.flow();
        assert!(FlowFixture::held(&flow, stored, FIRST).is_empty());
        assert_eq!(FlowFixture::held(&flow, later, FIRST), [0]);
    }

    #[test]
    fn a_target_rebuilt_from_itself_keeps_its_stores() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let rebuilt = fixture.defined_after(stored, NS, &[NS]);
        let later = fixture.read_after(rebuilt, NS);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, NS), [0]);
    }

    #[test]
    fn a_loop_binder_recurred_with_the_target_keeps_its_stores() {
        let mut fixture = FlowFixture::default();
        let entered = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let head = fixture.after(entered);
        let used = fixture.read_after(head, NS);
        let stored = fixture.stored_after(used, NS);
        let recurred = fixture.defined_after(stored, NS, &[NS]);
        fixture.graph.link(recurred, head);
        assert_eq!(FlowFixture::held(&fixture.flow(), head, NS), [0]);
    }

    #[test]
    fn a_target_built_from_another_target_takes_its_stores() {
        let mut fixture = FlowFixture::default();
        let other = fixture.defined_after(PointId::ENTRY, MS, &[]);
        let stored = fixture.stored_after(other, MS);
        let bound = fixture.defined_after(stored, NS, &[MS]);
        let again = fixture.stored_after(bound, NS);
        let later = fixture.read_after(again, NS);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, NS), [0, 1]);
    }

    #[test]
    fn a_loop_binder_entered_from_another_target_takes_its_stores() {
        let mut fixture = FlowFixture::default();
        let other = fixture.defined_after(PointId::ENTRY, MS, &[]);
        let stored = fixture.stored_after(other, MS);
        let entered = fixture.defined_after(stored, NS, &[MS]);
        let head = fixture.after(entered);
        let used = fixture.read_after(head, NS);
        let recurred = fixture.defined_after(used, NS, &[NS]);
        fixture.graph.link(recurred, head);
        assert_eq!(FlowFixture::held(&fixture.flow(), head, NS), [0]);
    }

    #[test]
    fn a_target_moved_away_leaves_its_stores_with_the_new_binding() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let moved = fixture.defined_after(stored, MS, &[NS]);
        let later = fixture.read_after(moved, MS);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, MS), [0]);
    }

    #[test]
    fn a_copy_takes_the_stores_of_its_original_but_never_gives_its_own_back() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let copied = fixture.defined_after(bound, MS, &[NS]);
        let stored = fixture.stored_after(copied, MS);
        let original = fixture.read_after(stored, NS);
        let copy = fixture.read_after(original, MS);
        let flow = fixture.flow();
        assert!(FlowFixture::held(&flow, original, NS).is_empty());
        assert_eq!(FlowFixture::held(&flow, copy, MS), [0]);
    }

    #[test]
    fn a_value_stored_from_another_target_carries_that_targets_stores() {
        let mut fixture = FlowFixture::default();
        let other = fixture.defined_after(PointId::ENTRY, MS, &[]);
        let first = fixture.stored_after(other, MS);
        let bound = fixture.defined_after(first, NS, &[]);
        let second = fixture.stored_from(bound, NS, &[MS]);
        let later = fixture.read_after(second, NS);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, NS), [0, 1]);
    }

    #[test]
    fn a_reset_to_a_fresh_value_drops_the_earlier_stores() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let reset = fixture.defined_after(stored, NS, &[]);
        let read = fixture.defined_after(reset, FIRST, &[NS]);
        let later = fixture.read_after(read, NS);
        let last = fixture.read_after(later, FIRST);
        let flow = fixture.flow();
        assert!(FlowFixture::held(&flow, later, NS).is_empty());
        assert!(FlowFixture::held(&flow, last, FIRST).is_empty());
    }

    #[test]
    fn a_store_after_a_reset_is_held() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let reset = fixture.defined_after(bound, NS, &[]);
        let stored = fixture.stored_after(reset, NS);
        let later = fixture.read_after(stored, NS);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, NS), [0]);
    }

    #[test]
    fn a_change_before_the_store_is_not_reached_by_it() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let changed = fixture.after(bound);
        let stored = fixture.stored_after(changed, NS);
        let later = fixture.read_after(stored, NS);
        let flow = fixture.flow();
        assert!(FlowFixture::held(&flow, changed, NS).is_empty());
        assert_eq!(FlowFixture::held(&flow, later, NS), [0]);
    }

    #[test]
    fn a_store_alive_on_one_arm_and_ended_on_the_other_is_alive_after_the_join() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let reset = fixture.defined_after(stored, NS, &[]);
        let kept = fixture.after(stored);
        let joined = fixture.graph.join(&[reset, kept]);
        let later = fixture.read_after(joined, NS);
        assert_eq!(FlowFixture::held(&fixture.flow(), later, NS), [0]);
    }

    #[test]
    fn a_store_ended_on_both_arms_is_gone_after_the_join() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let stored = fixture.stored_after(bound, NS);
        let reset = fixture.defined_after(stored, NS, &[]);
        let other = fixture.defined_after(stored, NS, &[]);
        let joined = fixture.graph.join(&[reset, other]);
        let later = fixture.read_after(joined, NS);
        assert!(FlowFixture::held(&fixture.flow(), later, NS).is_empty());
    }

    #[test]
    fn a_store_then_reset_on_one_arm_is_gone_after_the_join() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let armed = fixture.after(bound);
        let stored = fixture.stored_after(armed, NS);
        let used = fixture.read_after(stored, NS);
        let reset = fixture.defined_after(used, NS, &[]);
        let other = fixture.after(bound);
        let joined = fixture.graph.join(&[reset, other]);
        let later = fixture.read_after(joined, NS);
        let flow = fixture.flow();
        assert_eq!(FlowFixture::held(&flow, used, NS), [0]);
        assert!(FlowFixture::held(&flow, later, NS).is_empty());
    }

    #[test]
    fn a_reset_at_the_top_of_a_loop_ends_the_store_of_the_last_iteration() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let head = fixture.after(bound);
        let reset = fixture.defined_after(head, NS, &[]);
        let changed = fixture.after(reset);
        let stored = fixture.stored_after(changed, NS);
        let used = fixture.read_after(stored, NS);
        fixture.graph.link(used, head);
        let left = fixture.after(head);
        let end = fixture.read_after(left, NS);
        let flow = fixture.flow();
        assert!(FlowFixture::held(&flow, changed, NS).is_empty());
        assert_eq!(FlowFixture::held(&flow, used, NS), [0]);
        assert_eq!(FlowFixture::held(&flow, end, NS), [0]);
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
        let used = fixture.read_after(left, NS);
        fixture.graph.link(used, outer);
        let flow = fixture.flow();
        assert!(FlowFixture::held(&flow, bound, NS).is_empty());
        assert_eq!(FlowFixture::held(&flow, inner, NS), [0]);
        assert_eq!(FlowFixture::held(&flow, outer, NS), [0]);
    }

    #[test]
    fn a_binding_apart_from_every_target_holds_no_store() {
        let mut fixture = FlowFixture::default();
        let bound = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let apart = fixture.defined_after(bound, FIRST, &[]);
        let copied = fixture.defined_after(apart, MS, &[FIRST]);
        let stored = fixture.stored_after(copied, NS);
        let later = fixture.read_after(stored, NS);
        let copy = fixture.read_after(later, MS);
        let last = fixture.read_after(copy, FIRST);
        let flow = fixture.flow();
        assert_eq!(FlowFixture::held(&flow, later, NS), [0]);
        assert!(FlowFixture::held(&flow, copy, MS).is_empty());
        assert!(FlowFixture::held(&flow, last, FIRST).is_empty());
    }

    #[test]
    fn many_stores_in_one_body_each_last_until_the_next_reset() {
        const STORES: usize = 400;
        let mut fixture = FlowFixture::default();
        let mut point = fixture.defined_after(PointId::ENTRY, NS, &[]);
        let mut reads = Vec::new();
        for _ in 0..STORES {
            let stored = fixture.stored_after(point, NS);
            let read = fixture.read_after(stored, NS);
            reads.push(read);
            point = fixture.defined_after(read, NS, &[]);
        }
        let end = fixture.read_after(point, NS);
        let flow = fixture.flow();
        for (store, read) in reads.into_iter().enumerate() {
            assert_eq!(FlowFixture::held(&flow, read, NS), [store]);
        }
        assert!(FlowFixture::held(&flow, end, NS).is_empty());
    }
}
