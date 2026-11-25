use super::fact_row::FactRow;
use super::point_graph::PointGraph;
use super::point_id::PointId;

pub trait FactTransfer {
    fn transfer(&self, point: PointId, facts: &mut FactRow);
}

#[derive(Default)]
pub struct ForwardFacts {
    entering: Vec<FactRow>,
}

impl ForwardFacts {
    pub fn solved(graph: &PointGraph, facts: usize, transfer: &impl FactTransfer) -> Self {
        let mut solved = Self {
            entering: graph.points().map(|_| FactRow::sized(facts)).collect(),
        };
        while solved.grow(graph, transfer) {}
        solved
    }

    pub fn entering(&self, point: PointId, fact: usize) -> bool {
        self.entering_row(point)
            .is_some_and(|row| row.contains(fact))
    }

    pub fn entering_row(&self, point: PointId) -> Option<&FactRow> {
        self.entering.get(point.index())
    }

    fn grow(&mut self, graph: &PointGraph, transfer: &impl FactTransfer) -> bool {
        let mut grew = false;
        let mut leaving = FactRow::default();
        for (point, facts) in graph.points() {
            leaving.clone_from(&self.entering[point.index()]);
            transfer.transfer(point, &mut leaving);
            for successor in facts.successors() {
                grew |= self.entering[successor.index()].absorb(&leaving);
            }
        }
        grew
    }
}
