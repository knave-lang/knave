use std::collections::BTreeSet;

use super::loan_id::LoanId;
use crate::core::BindingId;

pub struct StoredLoans {
    pub sources: Vec<BindingId>,
    pub loans: BTreeSet<LoanId>,
}
