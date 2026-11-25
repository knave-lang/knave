use std::collections::VecDeque;

use rustc_hash::FxHashSet;

use super::indexed_points::IndexedPoints;
use super::point_facts::PointFacts;
use super::point_id::PointId;
use crate::core::BindingId;
use crate::source::Span;

pub struct PointGraph {
    facts: IndexedPoints,
}

impl PointGraph {
    pub fn new(facts: IndexedPoints) -> Self {
        Self { facts }
    }

    pub fn points(&self) -> impl DoubleEndedIterator<Item = (PointId, PointFacts<'_>)> + '_ {
        (0..self.facts.successors.points()).map(|index| {
            let point = PointId::new(index);
            (point, self.facts(point))
        })
    }

    pub fn first_use_after(&self, start: PointId, binding: BindingId) -> Option<Span> {
        let mut reached = FxHashSet::default();
        let mut pending: VecDeque<PointId> = self.facts(start).successors().collect();
        while let Some(point) = pending.pop_front() {
            if !reached.insert(point) {
                continue;
            }
            let facts = self.facts(point);
            if let Some(span) = facts.use_of(binding) {
                return Some(span);
            }
            pending.extend(facts.successors());
        }
        None
    }

    pub fn reachable_after(&self, start: PointId) -> FxHashSet<PointId> {
        let mut reached = FxHashSet::default();
        for successor in self.facts(start).successors() {
            reached.extend(self.reachable_from(successor));
        }
        reached
    }

    pub fn reachable_from(&self, start: PointId) -> FxHashSet<PointId> {
        let mut reached = FxHashSet::default();
        let mut pending = vec![start];
        while let Some(point) = pending.pop() {
            if reached.insert(point) {
                pending.extend(self.facts(point).successors());
            }
        }
        reached
    }

    fn facts(&self, point: PointId) -> PointFacts<'_> {
        PointFacts::new(&self.facts, point)
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn a_join_follows_each_point_before_it() {
        let mut graph = ControlFlowGraph::default();
        let left = graph.join(&[PointId::ENTRY]);
        let right = graph.join(&[PointId::ENTRY]);
        let joined = graph.join(&[left, right]);
        let successors: Vec<Vec<PointId>> = graph
            .finished()
            .points()
            .map(|(_, facts)| facts.successors().collect())
            .collect();
        assert_eq!(
            successors,
            [vec![left, right], vec![joined], vec![joined], vec![]]
        );
    }

    #[test]
    fn reaches_every_point_after_a_start_around_a_loop_but_none_before_it() {
        let mut graph = ControlFlowGraph::default();
        let head = graph.join(&[PointId::ENTRY]);
        let body = graph.join(&[head]);
        graph.link(body, head);
        let after = graph.join(&[head]);
        let reached = graph.finished().reachable_from(body);
        assert!(reached.contains(&head) && reached.contains(&after));
        assert!(!reached.contains(&PointId::ENTRY));
    }
}
