use std::collections::BTreeSet;

use rustc_hash::FxHashMap;

use super::holdings::{Holdings, Layer};
use super::loan_id::LoanId;
use crate::core::BindingId;

#[derive(Default)]
pub struct ValueRegions {
    loans: FxHashMap<(BindingId, Layer), BTreeSet<LoanId>>,
    sources: FxHashMap<(BindingId, Layer), BTreeSet<(BindingId, Layer)>>,
}

impl ValueRegions {
    pub fn flow_into(&mut self, binding: BindingId, held: &Holdings) {
        for layer in Layer::BOTH {
            let part = held.layer(layer);
            let key = (binding, layer);
            self.loans.entry(key).or_default().extend(part.loans());
            self.sources.entry(key).or_default().extend(part.values());
        }
    }

    pub fn resolved(mut self) -> Self {
        while self.grow() {}
        self
    }

    pub fn of(&self, binding: BindingId) -> impl Iterator<Item = LoanId> + '_ {
        Layer::BOTH
            .into_iter()
            .flat_map(move |layer| self.in_layer((binding, layer)))
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        Layer::BOTH
            .into_iter()
            .flat_map(|layer| {
                let part = held.layer(layer);
                part.loans()
                    .chain(part.values().flat_map(|source| self.in_layer(source)))
            })
            .collect()
    }

    fn in_layer(&self, key: (BindingId, Layer)) -> impl Iterator<Item = LoanId> + '_ {
        self.loans.get(&key).into_iter().flatten().copied()
    }

    fn grow(&mut self) -> bool {
        let additions: Vec<((BindingId, Layer), Vec<LoanId>)> = self
            .sources
            .iter()
            .map(|(key, sources)| (*key, self.missing_from(*key, sources)))
            .filter(|(_, missing)| !missing.is_empty())
            .collect();
        let grew = !additions.is_empty();
        for (key, missing) in additions {
            self.loans.entry(key).or_default().extend(missing);
        }
        grew
    }

    fn missing_from(
        &self,
        key: (BindingId, Layer),
        sources: &BTreeSet<(BindingId, Layer)>,
    ) -> Vec<LoanId> {
        let region = self.loans.get(&key);
        sources
            .iter()
            .flat_map(|source| self.in_layer(*source))
            .filter(|loan| !region.is_some_and(|loans| loans.contains(loan)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ValueRegions;
    use crate::core::BindingId;
    use crate::ownership::loans::holdings::Holdings;
    use crate::ownership::loans::loan_id::LoanId;

    struct RegionFixture;

    impl RegionFixture {
        fn value(index: u32) -> BindingId {
            BindingId::from_index(index)
        }

        fn loans_of(regions: &ValueRegions, index: u32) -> Vec<LoanId> {
            regions.of(Self::value(index)).collect()
        }
    }

    #[test]
    fn a_value_holds_the_loans_of_every_value_that_flows_into_it() {
        let mut regions = ValueRegions::default();
        regions.flow_into(
            RegionFixture::value(3),
            &Holdings::of_value(RegionFixture::value(2)),
        );
        regions.flow_into(
            RegionFixture::value(2),
            &Holdings::of_value(RegionFixture::value(1)),
        );
        regions.flow_into(RegionFixture::value(1), &Holdings::lent(LoanId::new(0)));
        let resolved = regions.resolved();
        assert_eq!(RegionFixture::loans_of(&resolved, 3), [LoanId::new(0)]);
    }

    #[test]
    fn the_loans_of_holdings_join_their_own_loans_and_their_values_regions() {
        let mut regions = ValueRegions::default();
        regions.flow_into(RegionFixture::value(1), &Holdings::lent(LoanId::new(0)));
        let held =
            Holdings::of_value(RegionFixture::value(1)).merged(Holdings::lent(LoanId::new(4)));
        let loans: Vec<LoanId> = regions.resolved().of_holdings(&held).into_iter().collect();
        assert_eq!(loans, [LoanId::new(0), LoanId::new(4)]);
    }
}
