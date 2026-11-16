use super::access::Move;
use super::control_flow_graph::ControlFlowGraph;
use super::holdings::{Holdings, Layer};
use super::lending::Lending;
use super::loan_place::LoanPlace;
use super::loan_table::LoanTable;
use super::place_access::PlaceAccess;
use super::point_id::PointId;
use crate::source::Span;

pub struct Lender<'l> {
    graph: &'l mut ControlFlowGraph,
    loans: &'l mut LoanTable,
    point: PointId,
}

impl<'l> Lender<'l> {
    pub fn new(graph: &'l mut ControlFlowGraph, loans: &'l mut LoanTable, point: PointId) -> Self {
        Self {
            graph,
            loans,
            point,
        }
    }

    pub fn lend(&mut self, lending: Lending) -> Holdings {
        let access = lending.access();
        let loan = self.loans.record(lending, self.point);
        self.access(access.lending(loan));
        Holdings::lent(loan)
    }

    pub fn move_out(&mut self, place: LoanPlace, span: Span) {
        self.access(PlaceAccess::new(place, &Move, span));
    }

    fn access(&mut self, access: PlaceAccess) {
        self.graph.point_mut(self.point).access(access);
    }

    pub fn receive(&mut self, passed: &Holdings) {
        let point = self.graph.point_mut(self.point);
        for layer in Layer::BOTH.map(|layer| passed.layer(layer)) {
            point.receive(layer.loans());
            for (value, _) in layer.values() {
                point.using(value);
            }
        }
    }
}
