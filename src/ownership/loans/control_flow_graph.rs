use rustc_hash::FxHashSet;

use super::point_id::PointId;
use super::program_point::{PointFacts, ProgramPoint};

pub struct ControlFlowGraph {
    points: Vec<ProgramPoint>,
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            points: vec![ProgramPoint::default()],
        }
    }

    pub fn join(&mut self, before: &[PointId]) -> PointId {
        let after = PointId::new(self.points.len());
        self.points.push(ProgramPoint::default());
        for point in before {
            self.link(*point, after);
        }
        after
    }

    pub fn link(&mut self, before: PointId, after: PointId) {
        self.points[before.index()].precede(after);
    }

    pub fn point_mut(&mut self, point: PointId) -> &mut ProgramPoint {
        &mut self.points[point.index()]
    }

    pub fn points(&self) -> impl DoubleEndedIterator<Item = (PointId, PointFacts<'_>)> + '_ {
        self.points
            .iter()
            .enumerate()
            .map(|(index, point)| (PointId::new(index), point.facts()))
    }

    pub fn reachable_from(&self, start: PointId) -> FxHashSet<PointId> {
        let mut reached = FxHashSet::default();
        let mut pending = vec![start];
        while let Some(point) = pending.pop() {
            if reached.insert(point) {
                pending.extend(self.points[point.index()].facts().successors());
            }
        }
        reached
    }
}

#[cfg(test)]
mod tests {
    use super::ControlFlowGraph;
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn a_join_follows_each_point_before_it() {
        let mut graph = ControlFlowGraph::new();
        let left = graph.join(&[PointId::ENTRY]);
        let right = graph.join(&[PointId::ENTRY]);
        let joined = graph.join(&[left, right]);
        let successors: Vec<Vec<PointId>> = graph
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
        let mut graph = ControlFlowGraph::new();
        let head = graph.join(&[PointId::ENTRY]);
        let body = graph.join(&[head]);
        graph.link(body, head);
        let after = graph.join(&[head]);
        let reached = graph.reachable_from(body);
        assert!(reached.contains(&head) && reached.contains(&after));
        assert!(!reached.contains(&PointId::ENTRY));
    }
}
