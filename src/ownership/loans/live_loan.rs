use super::loan::Loan;
use super::loan_conflict::LoanConflict;
use super::loan_holder::LoanHolder;
use super::loan_names::LoanNames;
use super::place_access::PlaceAccess;
use super::point_graph::PointGraph;
use super::point_id::PointId;

#[derive(Clone)]
pub struct LiveLoan<'b> {
    loan: &'b Loan,
    holder: LoanHolder,
}

impl<'b> LiveLoan<'b> {
    pub fn new(loan: &'b Loan, holder: LoanHolder) -> Self {
        Self { loan, holder }
    }

    pub fn conflict_with(
        &self,
        access: &'b PlaceAccess,
        at: PointId,
        graph: &PointGraph,
    ) -> Option<LoanConflict<'b>> {
        let conflict = LoanConflict::found(access, self.loan.lending(), self.holder)?;
        let later_use = self
            .holder
            .value()
            .and_then(|value| graph.first_use_after(at, value));
        Some(conflict.used_later_at(later_use))
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        format!(
            "{}, {}",
            self.loan.written(names),
            self.holder.written(names)
        )
    }
}
