use std::collections::BTreeSet;

use super::loan_id::LoanId;
use crate::core::BindingId;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Holdings {
    loans: BTreeSet<LoanId>,
    values: BTreeSet<BindingId>,
}

impl Holdings {
    pub fn of_loan(loan: LoanId) -> Self {
        Self {
            loans: BTreeSet::from([loan]),
            values: BTreeSet::new(),
        }
    }

    pub fn of_value(binding: BindingId) -> Self {
        Self {
            loans: BTreeSet::new(),
            values: BTreeSet::from([binding]),
        }
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.loans.extend(other.loans);
        self.values.extend(other.values);
        self
    }

    pub fn loans(&self) -> impl Iterator<Item = LoanId> + '_ {
        self.loans.iter().copied()
    }

    pub fn values(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.values.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::Holdings;
    use crate::core::BindingId;
    use crate::ownership::loans::loan_id::LoanId;

    #[test]
    fn merged_holdings_keep_the_loans_and_values_of_both() {
        let merged =
            Holdings::of_loan(LoanId::new(1)).merged(Holdings::of_value(BindingId::from_index(2)));
        assert_eq!(merged.loans().collect::<Vec<_>>(), [LoanId::new(1)]);
        assert_eq!(
            merged.values().collect::<Vec<_>>(),
            [BindingId::from_index(2)]
        );
    }
}
