use std::collections::BTreeSet;

use super::access_facts::AccessFacts;
use super::closure::Closure;
use super::control_flow_graph::ControlFlowGraph;
use super::holdings::Holdings;
use super::lent_place::LentPlace;
use super::live_loan::LiveLoan;
use super::live_values::LiveValues;
use super::loan_conflict::LoanConflict;
use super::loan_holder::LoanHolder;
use super::loan_id::LoanId;
use super::loan_table::LoanTable;
use super::point_id::PointId;
use super::program_point::PointFacts;
use super::result_loans::ResultLoans;
use super::value_regions::ValueRegions;

struct Liveness {
    values: LiveValues,
    result: ResultLoans,
}

pub struct BodyFacts<'w> {
    graph: ControlFlowGraph,
    loans: LoanTable,
    regions: ValueRegions,
    returned: Holdings,
    closures: Vec<Closure<'w>>,
}

impl<'w> BodyFacts<'w> {
    pub fn new(graph: ControlFlowGraph, loans: LoanTable, regions: ValueRegions) -> Self {
        Self {
            graph,
            loans,
            regions: regions.resolved(),
            returned: Holdings::default(),
            closures: Vec::new(),
        }
    }

    pub fn returning(self, returned: Holdings) -> Self {
        Self { returned, ..self }
    }

    pub fn with_closures(self, closures: Vec<Closure<'w>>) -> Self {
        Self { closures, ..self }
    }

    pub fn closures(&self) -> &[Closure<'w>] {
        &self.closures
    }

    pub fn accesses(&self) -> Vec<AccessFacts<'_>> {
        let liveness = self.liveness();
        self.graph
            .points()
            .filter(|(_, facts)| !facts.accesses().is_empty())
            .flat_map(|(point, facts)| {
                let loans = self.live_loans(point, &facts, &liveness);
                facts.accesses().iter().map(move |access| {
                    let met = loans
                        .iter()
                        .filter(|(loan, _)| !access.is_lent_as(*loan))
                        .map(|(loan, holder)| LiveLoan::new(self.loans.loan(*loan), *holder))
                        .collect();
                    AccessFacts::new(point, access, met)
                })
            })
            .collect()
    }

    pub fn escapes(&self) -> Vec<LentPlace<'_>> {
        self.regions
            .of_holdings(&self.returned)
            .into_iter()
            .map(|loan| self.loans.loan(loan).lending().lent())
            .collect()
    }

    pub fn conflicts(&self) -> Vec<LoanConflict<'_>> {
        self.accesses()
            .iter()
            .flat_map(|access| access.conflicts(&self.graph))
            .collect()
    }

    fn liveness(&self) -> Liveness {
        let returned = self.regions.of_holdings(&self.returned);
        Liveness {
            values: LiveValues::of(&self.graph),
            result: ResultLoans::of(&self.graph, &self.loans, &returned),
        }
    }

    fn live_loans(
        &self,
        point: PointId,
        facts: &PointFacts<'_>,
        liveness: &Liveness,
    ) -> Vec<(LoanId, LoanHolder)> {
        let mut values: Vec<_> = liveness.values.before(point).iter().copied().collect();
        values.sort_unstable();
        let held = values.into_iter().flat_map(|value| {
            self.regions
                .of(value)
                .into_iter()
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
        let mut listed = BTreeSet::new();
        let mut live: Vec<_> = held
            .chain(received)
            .chain(returned)
            .filter(|(loan, _)| listed.insert(*loan))
            .collect();
        live.sort_by_key(|(loan, _)| self.loans.loan(*loan).source_order());
        live
    }
}
