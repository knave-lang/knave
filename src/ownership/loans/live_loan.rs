use super::loan::Loan;
use super::loan_holder::LoanHolder;
use super::loan_names::LoanNames;

#[derive(Clone)]
pub struct LiveLoan<'b> {
    loan: &'b Loan,
    holder: LoanHolder,
}

impl<'b> LiveLoan<'b> {
    pub fn new(loan: &'b Loan, holder: LoanHolder) -> Self {
        Self { loan, holder }
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        format!(
            "{}, {}",
            self.loan.written(names),
            self.holder.written(names)
        )
    }
}
