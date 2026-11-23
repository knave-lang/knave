use super::indexed_points::IndexedPoints;
use super::loan_id::LoanId;
use super::place_access::PlaceAccess;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::point_lists::PointLists;
use super::value_use::ValueUse;
use crate::core::BindingId;
use crate::source::Span;

pub struct ControlFlowGraph {
    points: usize,
    defined: PointLists<BindingId>,
    used: PointLists<ValueUse>,
    received: PointLists<LoanId>,
    accesses: PointLists<PlaceAccess>,
    successors: PointLists<PointId>,
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self {
            points: 1,
            defined: PointLists::default(),
            used: PointLists::default(),
            received: PointLists::default(),
            accesses: PointLists::default(),
            successors: PointLists::default(),
        }
    }
}

impl ControlFlowGraph {
    pub fn join(&mut self, before: &[PointId]) -> PointId {
        let after = PointId::new(self.points);
        self.points += 1;
        for point in before {
            self.link(*point, after);
        }
        after
    }

    pub fn link(&mut self, before: PointId, after: PointId) {
        self.successors.push(before, after);
    }

    pub fn define(&mut self, point: PointId, binding: BindingId) {
        self.defined.push(point, binding);
    }

    pub fn using(&mut self, point: PointId, binding: BindingId, span: Span) {
        self.used.push(point, ValueUse { binding, span });
    }

    pub fn receive(&mut self, point: PointId, loans: impl IntoIterator<Item = LoanId>) {
        for loan in loans {
            self.received.push(point, loan);
        }
    }

    pub fn access(&mut self, point: PointId, access: PlaceAccess) {
        self.accesses.push(point, access);
    }

    pub fn finished(self) -> PointGraph {
        let points = self.points;
        PointGraph::new(IndexedPoints {
            defined: self.defined.indexed(points),
            used: self.used.indexed(points),
            received: self.received.indexed(points),
            accesses: self.accesses.indexed(points),
            successors: self.successors.indexed(points),
        })
    }
}
