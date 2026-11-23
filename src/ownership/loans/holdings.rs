use std::iter;

use super::held_value::HeldValue;
use super::loan_depth::LoanDepth;
use super::loan_id::LoanId;
use super::loan_layers::LoanLayers;
use super::sorted_set::SortedSet;
use crate::core::BindingId;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Holdings {
    loans: LoanLayers,
    values: SortedSet<HeldValue>,
}

impl Holdings {
    pub fn of_value(binding: BindingId) -> Self {
        Self {
            loans: LoanLayers::default(),
            values: iter::once(HeldValue::of(binding)).collect(),
        }
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.loans.extend(other.loans.loans());
        self.values.extend(other.values.iter());
        self
    }

    pub fn referent(&self) -> Self {
        Self {
            loans: self.loans.referent(),
            values: self.values.iter().map(HeldValue::referent).collect(),
        }
    }

    pub fn within(self) -> Self {
        Self {
            loans: self.loans.within(),
            values: self.values.iter().map(HeldValue::within).collect(),
        }
    }

    pub fn loans(&self) -> impl Iterator<Item = (LoanDepth, LoanId)> + '_ {
        self.loans.loans()
    }

    pub fn values(&self) -> impl Iterator<Item = HeldValue> + '_ {
        self.values.iter()
    }
}

impl From<LoanId> for Holdings {
    fn from(loan: LoanId) -> Self {
        Self {
            loans: iter::once((LoanDepth::TOP, loan)).collect(),
            values: SortedSet::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Holdings;
    use crate::core::BindingId;
    use crate::ownership::loans::held_value::HeldValue;
    use crate::ownership::loans::loan_depth::LoanDepth;
    use crate::ownership::loans::loan_id::LoanId;

    struct HoldingsFixture;

    impl HoldingsFixture {
        fn layer(holdings: &Holdings, depth: usize) -> Vec<LoanId> {
            holdings
                .loans()
                .filter(|(at, _)| *at == LoanDepth::at(depth))
                .map(|(_, loan)| loan)
                .collect()
        }

        fn values(holdings: &Holdings) -> Vec<HeldValue> {
            holdings.values().collect()
        }
    }

    #[test]
    fn a_loan_is_the_top_layer_and_a_value_keeps_its_own_layers() {
        let value = BindingId::from_index(2);
        let merged = Holdings::from(LoanId::new(1)).merged(Holdings::of_value(value));
        assert_eq!(HoldingsFixture::layer(&merged, 0), [LoanId::new(1)]);
        assert_eq!(HoldingsFixture::values(&merged), [HeldValue::of(value)]);
    }

    #[test]
    fn the_referent_drops_the_top_layer() {
        let value = BindingId::from_index(2);
        let referent = Holdings::from(LoanId::new(1))
            .merged(Holdings::of_value(value))
            .referent();
        assert_eq!(referent.loans().count(), 0);
        assert_eq!(
            HoldingsFixture::values(&referent),
            [HeldValue::of(value).referent()]
        );
    }

    #[test]
    fn each_reference_a_loan_is_held_within_keeps_it_one_layer_deeper() {
        let twice = Holdings::from(LoanId::new(1))
            .within()
            .merged(Holdings::from(LoanId::new(2)))
            .within();
        assert_eq!(HoldingsFixture::layer(&twice, 1), [LoanId::new(2)]);
        assert_eq!(HoldingsFixture::layer(&twice, 2), [LoanId::new(1)]);
        assert_eq!(
            HoldingsFixture::layer(&twice.referent().referent(), 0),
            [LoanId::new(1)]
        );
    }
}
