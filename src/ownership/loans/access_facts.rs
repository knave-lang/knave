use super::loan::Loan;
use super::loan_names::LoanNames;
use super::place_access::PlaceAccess;
use crate::core::BindingId;

pub struct AccessFacts<'b> {
    access: &'b PlaceAccess,
    live: Vec<LiveLoan<'b>>,
}

#[derive(Clone)]
pub struct LiveLoan<'b> {
    loan: &'b Loan,
    holder: LoanHolder,
}

#[derive(Clone, Copy)]
pub enum LoanHolder {
    Value(BindingId),
    Call,
    Result,
}

impl<'b> AccessFacts<'b> {
    pub fn new(access: &'b PlaceAccess, live: Vec<LiveLoan<'b>>) -> Self {
        Self { access, live }
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        let live: Vec<String> = self.live.iter().map(|loan| loan.written(names)).collect();
        let live = if live.is_empty() {
            "no live loan".to_string()
        } else {
            live.join("; ")
        };
        format!(
            "{} {}: {live}",
            names.position(self.access.span()),
            self.access.written(names.bindings())
        )
    }
}

impl<'b> LiveLoan<'b> {
    pub fn new(loan: &'b Loan, holder: LoanHolder) -> Self {
        Self { loan, holder }
    }

    fn written(&self, names: &LoanNames<'_>) -> String {
        let holder = match self.holder {
            LoanHolder::Value(binding) => {
                format!("held by `{}`", names.bindings().get(binding).name())
            }
            LoanHolder::Call => "passed to the call".to_string(),
            LoanHolder::Result => "returned".to_string(),
        };
        format!(
            "{} lent at {}, {holder}",
            self.loan.written(names.bindings()),
            names.position(self.loan.span())
        )
    }
}
