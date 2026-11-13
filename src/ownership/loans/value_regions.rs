use std::collections::BTreeSet;

use rustc_hash::FxHashMap;

use super::holdings::Holdings;
use super::loan_id::LoanId;
use crate::core::BindingId;

#[derive(Default)]
pub struct ValueRegions {
    loans: FxHashMap<BindingId, BTreeSet<LoanId>>,
    sources: FxHashMap<BindingId, BTreeSet<BindingId>>,
}

impl ValueRegions {
    pub fn flow_into(&mut self, binding: BindingId, held: &Holdings) {
        self.loans.entry(binding).or_default().extend(held.loans());
        self.sources
            .entry(binding)
            .or_default()
            .extend(held.values());
    }

    pub fn resolved(mut self) -> Self {
        while self.grow() {}
        self
    }

    pub fn of(&self, binding: BindingId) -> impl Iterator<Item = LoanId> + '_ {
        self.loans.get(&binding).into_iter().flatten().copied()
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        held.loans()
            .chain(held.values().flat_map(|binding| self.of(binding)))
            .collect()
    }

    fn grow(&mut self) -> bool {
        let additions: Vec<(BindingId, Vec<LoanId>)> = self
            .sources
            .iter()
            .map(|(binding, sources)| (*binding, self.missing_from(*binding, sources)))
            .filter(|(_, missing)| !missing.is_empty())
            .collect();
        let grew = !additions.is_empty();
        for (binding, missing) in additions {
            self.loans.entry(binding).or_default().extend(missing);
        }
        grew
    }

    fn missing_from(&self, binding: BindingId, sources: &BTreeSet<BindingId>) -> Vec<LoanId> {
        let region = self.loans.get(&binding);
        sources
            .iter()
            .flat_map(|source| self.of(*source))
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
        regions.flow_into(RegionFixture::value(1), &Holdings::of_loan(LoanId::new(0)));
        let resolved = regions.resolved();
        assert_eq!(RegionFixture::loans_of(&resolved, 3), [LoanId::new(0)]);
    }

    #[test]
    fn the_loans_of_holdings_join_their_own_loans_and_their_values_regions() {
        let mut regions = ValueRegions::default();
        regions.flow_into(RegionFixture::value(1), &Holdings::of_loan(LoanId::new(0)));
        let held =
            Holdings::of_value(RegionFixture::value(1)).merged(Holdings::of_loan(LoanId::new(4)));
        let loans: Vec<LoanId> = regions.resolved().of_holdings(&held).into_iter().collect();
        assert_eq!(loans, [LoanId::new(0), LoanId::new(4)]);
    }
}
