use super::control_flow_graph::ControlFlowGraph;
use super::holdings::Holdings;
use super::lending::Lending;
use super::loan_table::LoanTable;
use super::place_access::PlaceAccess;
use super::point_id::PointId;

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
        self.access(lending.access());
        Holdings::of_loan(self.loans.record(lending, self.point))
    }

    pub fn access(&mut self, access: PlaceAccess) {
        self.graph.point_mut(self.point).access(access);
    }

    pub fn receive(&mut self, passed: &Holdings) {
        let point = self.graph.point_mut(self.point);
        point.receive(passed.loans());
        for value in passed.values() {
            point.using(value);
        }
    }
}
