use super::lending::Lending;
use super::loan::Loan;
use super::loan_id::LoanId;
use super::point_id::PointId;

#[derive(Default)]
pub struct LoanTable {
    loans: Vec<Loan>,
}

impl LoanTable {
    pub fn record(&mut self, lending: Lending, point: PointId) -> LoanId {
        let loan = LoanId::new(self.loans.len());
        self.loans.push(Loan::new(lending, point));
        loan
    }

    pub fn loan(&self, loan: LoanId) -> &Loan {
        &self.loans[loan.index()]
    }
}
