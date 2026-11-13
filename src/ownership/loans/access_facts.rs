use super::live_loan::LiveLoan;
use super::loan_names::LoanNames;
use super::place_access::PlaceAccess;

pub struct AccessFacts<'b> {
    access: &'b PlaceAccess,
    live: Vec<LiveLoan<'b>>,
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
        format!("{}: {live}", self.access.written(names))
    }
}
