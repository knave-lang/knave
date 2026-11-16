use std::collections::BTreeSet;

use super::loan_id::LoanId;
use crate::core::BindingId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    Reference,
    Referent,
}

impl Layer {
    pub const BOTH: [Self; 2] = [Self::Reference, Self::Referent];
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct HeldLayer {
    loans: BTreeSet<LoanId>,
    values: BTreeSet<(BindingId, Layer)>,
}

impl HeldLayer {
    pub fn loans(&self) -> impl Iterator<Item = LoanId> + '_ {
        self.loans.iter().copied()
    }

    pub fn values(&self) -> impl Iterator<Item = (BindingId, Layer)> + '_ {
        self.values.iter().copied()
    }

    fn merged(mut self, other: Self) -> Self {
        self.loans.extend(other.loans);
        self.values.extend(other.values);
        self
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Holdings {
    reference: HeldLayer,
    referent: HeldLayer,
}

impl Holdings {
    pub fn lent(loan: LoanId) -> Self {
        Self {
            reference: HeldLayer {
                loans: BTreeSet::from([loan]),
                values: BTreeSet::new(),
            },
            referent: HeldLayer::default(),
        }
    }

    pub fn of_value(binding: BindingId) -> Self {
        let layer = |layer| HeldLayer {
            loans: BTreeSet::new(),
            values: BTreeSet::from([(binding, layer)]),
        };
        Self {
            reference: layer(Layer::Reference),
            referent: layer(Layer::Referent),
        }
    }

    pub fn merged(self, other: Self) -> Self {
        Self {
            reference: self.reference.merged(other.reference),
            referent: self.referent.merged(other.referent),
        }
    }

    pub fn referent(&self) -> Self {
        Self {
            reference: HeldLayer::default(),
            referent: self.referent.clone(),
        }
    }

    pub fn within(self) -> Self {
        Self {
            reference: HeldLayer::default(),
            referent: self.reference.merged(self.referent),
        }
    }

    pub fn layer(&self, layer: Layer) -> &HeldLayer {
        match layer {
            Layer::Reference => &self.reference,
            Layer::Referent => &self.referent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Holdings, Layer};
    use crate::core::BindingId;
    use crate::ownership::loans::loan_id::LoanId;

    struct HoldingsFixture;

    impl HoldingsFixture {
        fn loans(holdings: &Holdings, layer: Layer) -> Vec<LoanId> {
            holdings.layer(layer).loans().collect()
        }

        fn values(holdings: &Holdings, layer: Layer) -> Vec<(BindingId, Layer)> {
            holdings.layer(layer).values().collect()
        }
    }

    #[test]
    fn a_loan_is_the_reference_layer_and_a_value_reaches_both_layers() {
        let value = BindingId::from_index(2);
        let merged = Holdings::lent(LoanId::new(1)).merged(Holdings::of_value(value));
        assert_eq!(
            HoldingsFixture::loans(&merged, Layer::Reference),
            [LoanId::new(1)]
        );
        assert_eq!(HoldingsFixture::loans(&merged, Layer::Referent), []);
        assert_eq!(
            HoldingsFixture::values(&merged, Layer::Reference),
            [(value, Layer::Reference)]
        );
        assert_eq!(
            HoldingsFixture::values(&merged, Layer::Referent),
            [(value, Layer::Referent)]
        );
    }

    #[test]
    fn the_referent_drops_the_reference_layer() {
        let value = BindingId::from_index(2);
        let referent = Holdings::lent(LoanId::new(1))
            .merged(Holdings::of_value(value))
            .referent();
        assert_eq!(HoldingsFixture::loans(&referent, Layer::Reference), []);
        assert_eq!(HoldingsFixture::values(&referent, Layer::Reference), []);
        assert_eq!(
            HoldingsFixture::values(&referent, Layer::Referent),
            [(value, Layer::Referent)]
        );
    }

    #[test]
    fn a_value_held_within_another_keeps_both_layers_beneath_it() {
        let value = BindingId::from_index(2);
        let within = Holdings::of_value(value).within();
        assert_eq!(
            HoldingsFixture::values(&within, Layer::Referent),
            [(value, Layer::Reference), (value, Layer::Referent)]
        );
        let within = Holdings::lent(LoanId::new(1)).within();
        assert_eq!(HoldingsFixture::loans(&within, Layer::Reference), []);
        assert_eq!(
            HoldingsFixture::loans(&within, Layer::Referent),
            [LoanId::new(1)]
        );
    }
}
