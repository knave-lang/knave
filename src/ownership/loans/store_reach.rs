use rustc_hash::FxHashSet;

use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use crate::core::BindingId;

#[derive(Default)]
pub struct StoreReach {
    reaching: Vec<FxHashSet<usize>>,
}

#[derive(Clone, Default)]
struct StoreChange {
    stored: Vec<usize>,
    renewed: Vec<usize>,
}

impl StoreReach {
    pub fn of(
        graph: &PointGraph,
        stores: &[RecordedStore],
        renewals: &[(PointId, BindingId)],
    ) -> Self {
        let mut reach = Self {
            reaching: graph.points().map(|_| FxHashSet::default()).collect(),
        };
        let changes = Self::changes(reach.reaching.len(), stores, renewals);
        while reach.grow(graph, &changes) {}
        reach
    }

    pub fn reaches(&self, point: PointId, store: usize) -> bool {
        self.reaching[point.index()].contains(&store)
    }

    fn changes(
        points: usize,
        stores: &[RecordedStore],
        renewals: &[(PointId, BindingId)],
    ) -> Vec<StoreChange> {
        let mut changes = vec![StoreChange::default(); points];
        for (index, store) in stores.iter().enumerate() {
            changes[store.point.index()].stored.push(index);
        }
        for (point, renewed) in renewals {
            let ended = stores
                .iter()
                .enumerate()
                .filter(|(_, store)| store.target == *renewed)
                .map(|(index, _)| index);
            changes[point.index()].renewed.extend(ended);
        }
        changes
    }

    fn grow(&mut self, graph: &PointGraph, changes: &[StoreChange]) -> bool {
        let mut grew = false;
        for (point, facts) in graph.points() {
            let leaving = changes[point.index()].applied_to(&self.reaching[point.index()]);
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

impl StoreChange {
    fn applied_to(&self, reaching: &FxHashSet<usize>) -> Vec<usize> {
        reaching
            .iter()
            .filter(|store| !self.renewed.contains(store))
            .chain(&self.stored)
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::StoreReach;
    use crate::core::BindingId;
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::holdings::Holdings;
    use crate::ownership::loans::point_id::PointId;
    use crate::ownership::loans::recorded_store::RecordedStore;

    struct ReachFixture {
        graph: ControlFlowGraph,
        target: BindingId,
    }

    impl ReachFixture {
        fn new() -> Self {
            Self {
                graph: ControlFlowGraph::default(),
                target: BindingId::from_index(3),
            }
        }

        fn after(&mut self, point: PointId) -> PointId {
            self.graph.join(&[point])
        }

        fn reach(self, stored: PointId, renewals: &[(PointId, BindingId)]) -> StoreReach {
            let store = RecordedStore {
                point: stored,
                target: self.target,
                held: Holdings::default(),
            };
            StoreReach::of(&self.graph.finished(), &[store], renewals)
        }
    }

    #[test]
    fn a_store_reaches_the_points_after_it_until_its_target_is_renewed() {
        let mut fixture = ReachFixture::new();
        let target = fixture.target;
        let before = fixture.after(PointId::ENTRY);
        let stored = fixture.after(before);
        let after = fixture.after(stored);
        let renewed = fixture.after(after);
        let later = fixture.after(renewed);
        let reach = fixture.reach(stored, &[(renewed, target)]);
        assert!(!reach.reaches(before, 0));
        assert!(!reach.reaches(stored, 0));
        assert!(reach.reaches(after, 0));
        assert!(reach.reaches(renewed, 0));
        assert!(!reach.reaches(later, 0));
    }

    #[test]
    fn a_renewal_of_another_local_leaves_the_store_in_place() {
        let mut fixture = ReachFixture::new();
        let stored = fixture.after(PointId::ENTRY);
        let renewed = fixture.after(stored);
        let later = fixture.after(renewed);
        let reach = fixture.reach(stored, &[(renewed, BindingId::from_index(4))]);
        assert!(reach.reaches(later, 0));
    }

    #[test]
    fn a_store_in_a_loop_reaches_the_loop_head_again() {
        let mut fixture = ReachFixture::new();
        let head = fixture.after(PointId::ENTRY);
        let stored = fixture.after(head);
        fixture.graph.link(stored, head);
        let reach = fixture.reach(stored, &[]);
        assert!(reach.reaches(head, 0));
    }
}
