use std::collections::BTreeSet;
use std::iter;

use super::held_value::HeldValue;
use super::loan_id::LoanId;
use crate::core::BindingId;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Holdings {
    layers: Vec<BTreeSet<LoanId>>,
    values: BTreeSet<HeldValue>,
}

impl Holdings {
    pub fn lent(loan: LoanId) -> Self {
        Self {
            layers: vec![BTreeSet::from([loan])],
            values: BTreeSet::new(),
        }
    }

    pub fn of_value(binding: BindingId) -> Self {
        Self {
            layers: Vec::new(),
            values: BTreeSet::from([HeldValue::of(binding)]),
        }
    }

    pub fn merged(mut self, other: Self) -> Self {
        for (depth, loans) in other.layers.into_iter().enumerate() {
            self.layer_mut(depth).extend(loans);
        }
        self.values.extend(other.values);
        self
    }

    pub fn referent(&self) -> Self {
        Self {
            layers: self.layers.iter().skip(1).cloned().collect(),
            values: self.values.iter().map(|value| value.referent()).collect(),
        }
    }

    pub fn within(self) -> Self {
        Self {
            layers: iter::once(BTreeSet::new()).chain(self.layers).collect(),
            values: self.values.into_iter().map(HeldValue::within).collect(),
        }
    }

    pub fn loans(&self) -> impl Iterator<Item = (usize, LoanId)> + '_ {
        self.layers
            .iter()
            .enumerate()
            .flat_map(|(depth, loans)| loans.iter().map(move |loan| (depth, *loan)))
    }

    pub fn values(&self) -> impl Iterator<Item = HeldValue> + '_ {
        self.values.iter().copied()
    }

    fn layer_mut(&mut self, depth: usize) -> &mut BTreeSet<LoanId> {
        if self.layers.len() <= depth {
            self.layers.resize_with(depth + 1, BTreeSet::new);
        }
        &mut self.layers[depth]
    }
}

#[cfg(test)]
mod tests {
    use super::Holdings;
    use crate::core::BindingId;
    use crate::ownership::loans::held_value::HeldValue;
    use crate::ownership::loans::loan_id::LoanId;

    struct HoldingsFixture;

    impl HoldingsFixture {
        fn layer(holdings: &Holdings, depth: usize) -> Vec<LoanId> {
            holdings
                .loans()
                .filter(|(at, _)| *at == depth)
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
        let merged = Holdings::lent(LoanId::new(1)).merged(Holdings::of_value(value));
        assert_eq!(HoldingsFixture::layer(&merged, 0), [LoanId::new(1)]);
        assert_eq!(HoldingsFixture::values(&merged), [HeldValue::of(value)]);
    }

    #[test]
    fn the_referent_drops_the_top_layer() {
        let value = BindingId::from_index(2);
        let referent = Holdings::lent(LoanId::new(1))
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
        let twice = Holdings::lent(LoanId::new(1))
            .within()
            .merged(Holdings::lent(LoanId::new(2)))
            .within();
        assert_eq!(HoldingsFixture::layer(&twice, 1), [LoanId::new(2)]);
        assert_eq!(HoldingsFixture::layer(&twice, 2), [LoanId::new(1)]);
        assert_eq!(
            HoldingsFixture::layer(&twice.referent().referent(), 0),
            [LoanId::new(1)]
        );
    }
}
