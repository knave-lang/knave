use super::access_facts::{AccessFacts, LiveLoan, LoanHolder};
use super::body_graph::BodyGraph;
use super::holdings::Holdings;
use super::live_values::LiveValues;
use super::point_id::PointId;
use super::program_point::PointFacts;
use super::result_loans::ResultLoans;
use super::value_regions::ValueRegions;
use crate::types::TypedExpr;

struct Liveness {
    values: LiveValues,
    result: ResultLoans,
}

pub struct WalkedBody<'w> {
    graph: BodyGraph,
    regions: ValueRegions,
    returned: Holdings,
    closures: Vec<&'w TypedExpr>,
}

impl<'w> WalkedBody<'w> {
    pub fn new(graph: BodyGraph, regions: ValueRegions) -> Self {
        Self {
            graph,
            regions: regions.resolved(),
            returned: Holdings::default(),
            closures: Vec::new(),
        }
    }

    pub fn returning(self, returned: Holdings) -> Self {
        Self { returned, ..self }
    }

    pub fn with_closures(self, closures: Vec<&'w TypedExpr>) -> Self {
        Self { closures, ..self }
    }

    pub fn closures(&self) -> &[&'w TypedExpr] {
        &self.closures
    }

    pub fn accesses(&self) -> Vec<AccessFacts<'_>> {
        let liveness = Liveness {
            values: LiveValues::of(&self.graph),
            result: ResultLoans::of(&self.graph, &self.regions.of_holdings(&self.returned)),
        };
        self.graph
            .points()
            .flat_map(|(point, facts)| {
                let loans = self.live_loans(point, &facts, &liveness);
                facts
                    .accesses()
                    .iter()
                    .map(move |access| AccessFacts::new(access, loans.clone()))
            })
            .collect()
    }

    fn live_loans(
        &self,
        point: PointId,
        facts: &PointFacts<'_>,
        liveness: &Liveness,
    ) -> Vec<LiveLoan<'_>> {
        let mut values: Vec<_> = liveness.values.before(point).iter().copied().collect();
        values.sort_unstable();
        let held = values.into_iter().flat_map(|value| {
            self.regions
                .of(value)
                .map(move |loan| (loan, LoanHolder::Value(value)))
        });
        let received = facts
            .received()
            .iter()
            .map(|loan| (*loan, LoanHolder::Call));
        let returned = liveness
            .result
            .live_at(point)
            .map(|loan| (loan, LoanHolder::Result));
        held.chain(received)
            .chain(returned)
            .filter(|(loan, _)| self.graph.loan(*loan).made_at() != point)
            .map(|(loan, holder)| LiveLoan::new(self.graph.loan(loan), holder))
            .collect()
    }
}
