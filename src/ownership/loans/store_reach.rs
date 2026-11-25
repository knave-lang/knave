use rustc_hash::{FxHashMap, FxHashSet};

use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use crate::core::BindingId;

pub struct StoreReach<H> {
    stores: Vec<RecordedStore<H>>,
    by_target: FxHashMap<BindingId, Vec<usize>>,
    reaching: Vec<FxHashSet<usize>>,
}

impl<H> StoreReach<H> {
    pub fn of(graph: &PointGraph, stores: Vec<RecordedStore<H>>) -> Self {
        let mut reach = Self {
            by_target: Self::indexed_by_target(&stores),
            reaching: Vec::new(),
            stores,
        };
        if !reach.stores.is_empty() {
            reach.reaching = graph.points().map(|_| FxHashSet::default()).collect();
            let stored = reach.stored_at_each_point();
            while reach.grow(graph, &stored) {}
        }
        reach
    }

    pub fn stores_into(&self, target: BindingId) -> bool {
        self.by_target.contains_key(&target)
    }

    pub fn held_at(&self, point: PointId, target: BindingId) -> impl Iterator<Item = &H> + '_ {
        let reaching = self.reaching.get(point.index());
        self.by_target
            .get(&target)
            .into_iter()
            .flatten()
            .filter(move |store| reaching.is_some_and(|reaching| reaching.contains(store)))
            .map(|store| &self.stores[*store].held)
    }

    fn indexed_by_target(stores: &[RecordedStore<H>]) -> FxHashMap<BindingId, Vec<usize>> {
        let mut by_target: FxHashMap<BindingId, Vec<usize>> = FxHashMap::default();
        for (index, store) in stores.iter().enumerate() {
            by_target.entry(store.target).or_default().push(index);
        }
        by_target
    }

    fn stored_at_each_point(&self) -> Vec<Vec<usize>> {
        let mut stored = vec![Vec::new(); self.reaching.len()];
        for (index, store) in self.stores.iter().enumerate() {
            stored[store.point.index()].push(index);
        }
        stored
    }

    fn grow(&mut self, graph: &PointGraph, stored: &[Vec<usize>]) -> bool {
        let mut grew = false;
        for (point, facts) in graph.points() {
            let leaving: Vec<usize> = self.reaching[point.index()]
                .iter()
                .filter(|store| !facts.renews(self.stores[**store].target))
                .chain(&stored[point.index()])
                .copied()
                .collect();
            for successor in facts.successors() {
                let entering = &mut self.reaching[successor.index()];
                for store in &leaving {
                    grew |= entering.insert(*store);
                }
            }
        }
        grew
    }
}

#[cfg(test)]
mod tests {
    use super::StoreReach;
    use crate::core::BindingId;
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::new_value::NewValue;
    use crate::ownership::loans::point_id::PointId;
    use crate::ownership::loans::recorded_store::RecordedStore;

    struct ReachFixture {
        graph: ControlFlowGraph,
        stores: Vec<RecordedStore<usize>>,
    }

    impl ReachFixture {
        fn new() -> Self {
            Self {
                graph: ControlFlowGraph::default(),
                stores: Vec::new(),
            }
        }

        fn after(&mut self, point: PointId) -> PointId {
            self.graph.join(&[point])
        }

        fn store(&mut self, point: PointId, target: BindingId) {
            let held = self.stores.len();
            self.stores.push(RecordedStore { point, target, held });
        }

        fn replace(&mut self, point: PointId, target: BindingId) {
            self.graph.define(point, NewValue::Replaced(target));
        }

        fn reach(self) -> StoreReach<usize> {
            StoreReach::of(&self.graph.finished(), self.stores)
        }
    }

    fn held(reach: &StoreReach<usize>, point: PointId, target: BindingId) -> Vec<usize> {
        reach.held_at(point, target).copied().collect()
    }

    #[test]
    fn a_store_reaches_the_points_after_it_until_its_target_is_replaced() {
        let target = BindingId::from_index(3);
        let mut fixture = ReachFixture::new();
        let before = fixture.after(PointId::ENTRY);
        let stored = fixture.after(before);
        let after = fixture.after(stored);
        let replaced = fixture.after(after);
        let later = fixture.after(replaced);
        fixture.store(stored, target);
        fixture.replace(replaced, target);
        let reach = fixture.reach();
        assert!(held(&reach, before, target).is_empty());
        assert!(held(&reach, stored, target).is_empty());
        assert_eq!(held(&reach, after, target), [0]);
        assert_eq!(held(&reach, replaced, target), [0]);
        assert!(held(&reach, later, target).is_empty());
    }

    #[test]
    fn a_new_binding_of_the_target_ends_its_stores() {
        let target = BindingId::from_index(3);
        let mut fixture = ReachFixture::new();
        let stored = fixture.after(PointId::ENTRY);
        let bound = fixture.after(stored);
        let later = fixture.after(bound);
        fixture.store(stored, target);
        fixture.graph.define(bound, NewValue::Bound(target));
        assert!(held(&fixture.reach(), later, target).is_empty());
    }

    #[test]
    fn a_replacement_of_another_local_leaves_the_store_in_place() {
        let target = BindingId::from_index(3);
        let mut fixture = ReachFixture::new();
        let stored = fixture.after(PointId::ENTRY);
        let replaced = fixture.after(stored);
        let later = fixture.after(replaced);
        fixture.store(stored, target);
        fixture.replace(replaced, BindingId::from_index(4));
        let reach = fixture.reach();
        assert!(reach.stores_into(target));
        assert_eq!(held(&reach, later, target), [0]);
    }

    #[test]
    fn a_store_in_a_loop_reaches_the_loop_head_again() {
        let target = BindingId::from_index(3);
        let mut fixture = ReachFixture::new();
        let head = fixture.after(PointId::ENTRY);
        let stored = fixture.after(head);
        fixture.graph.link(stored, head);
        fixture.store(stored, target);
        assert_eq!(held(&fixture.reach(), head, target), [0]);
    }

    #[test]
    fn many_stores_in_one_body_each_reach_only_until_their_target_is_replaced() {
        const STORES: u32 = 400;
        let mut fixture = ReachFixture::new();
        let mut point = PointId::ENTRY;
        let mut probes = Vec::new();
        for index in 0..STORES {
            let target = BindingId::from_index(index);
            let stored = fixture.after(point);
            let between = fixture.after(stored);
            let replaced = fixture.after(between);
            fixture.store(stored, target);
            fixture.replace(replaced, target);
            probes.push((target, between, replaced));
            point = replaced;
        }
        let end = fixture.after(point);
        let reach = fixture.reach();
        for (index, (target, between, replaced)) in probes.into_iter().enumerate() {
            assert_eq!(held(&reach, between, target), [index]);
            assert_eq!(held(&reach, replaced, target), [index]);
            assert!(held(&reach, end, target).is_empty());
        }
    }
}
