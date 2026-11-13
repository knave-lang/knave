use super::lending::Lending;
use super::loan_names::LoanNames;
use super::point_id::PointId;

pub struct Loan {
    lending: Lending,
    made_at: PointId,
}

impl Loan {
    pub fn new(lending: Lending, made_at: PointId) -> Self {
        Self { lending, made_at }
    }

    pub fn made_at(&self) -> PointId {
        self.made_at
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        self.lending.written(names)
    }
}
