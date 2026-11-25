use std::collections::BTreeSet;

use rustc_hash::FxHashMap;

use super::held_value::HeldValue;
use super::holdings::Holdings;
use super::loan_depth::LoanDepth;
use super::loan_id::LoanId;
use super::loan_layers::LoanLayers;
use super::recorded_flow::RecordedFlow;
use crate::core::BindingId;

#[derive(Clone, Default)]
pub struct ValueRegions {
    loans: FxHashMap<BindingId, LoanLayers>,
    sources: FxHashMap<BindingId, BTreeSet<HeldValue>>,
}

impl ValueRegions {
    pub fn of_flows(flows: &[RecordedFlow]) -> Self {
        let mut regions = Self::default();
        for flow in flows {
            regions.flow_into(flow.binding, &flow.held);
        }
        regions.resolved()
    }

    fn flow_into(&mut self, binding: BindingId, held: &Holdings) {
        self.loans.entry(binding).or_default().extend(held.loans());
        self.sources
            .entry(binding)
            .or_default()
            .extend(held.values());
    }

    fn resolved(mut self) -> Self {
        while self.grow() {}
        self
    }

    pub fn of(&self, binding: BindingId) -> BTreeSet<LoanId> {
        self.layers_of(binding).map(|(_, loan)| loan).collect()
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        held.loans()
            .chain(held.values().flat_map(|value| self.placed(value)))
            .map(|(_, loan)| loan)
            .collect()
    }

    fn layers_of(&self, binding: BindingId) -> impl Iterator<Item = (LoanDepth, LoanId)> + '_ {
        self.loans
            .get(&binding)
            .into_iter()
            .flat_map(LoanLayers::loans)
    }

    fn placed(&self, value: HeldValue) -> impl Iterator<Item = (LoanDepth, LoanId)> + '_ {
        self.layers_of(value.binding())
            .filter_map(move |(depth, loan)| Some((value.placed(depth)?, loan)))
    }

    fn grow(&mut self) -> bool {
        let regions: &Self = self;
        let additions: Vec<(BindingId, LoanDepth, LoanId)> = regions
            .sources
            .iter()
            .flat_map(|(binding, sources)| {
                sources
                    .iter()
                    .flat_map(move |source| regions.missing_from(*binding, *source))
            })
            .collect();
        let grew = !additions.is_empty();
        for (binding, depth, loan) in additions {
            self.loans.entry(binding).or_default().insert(depth, loan);
        }
        grew
    }

    fn missing_from(
        &self,
        binding: BindingId,
        source: HeldValue,
    ) -> impl Iterator<Item = (BindingId, LoanDepth, LoanId)> + '_ {
        self.placed(source)
            .filter(move |(depth, loan)| !self.holds(binding, *depth, *loan))
            .map(move |(depth, loan)| (binding, depth, loan))
    }

    fn holds(&self, binding: BindingId, depth: LoanDepth, loan: LoanId) -> bool {
        self.loans
            .get(&binding)
            .is_some_and(|layers| layers.holds(depth, loan))
    }
}

#[cfg(test)]
mod tests {
    use super::ValueRegions;
    use crate::core::BindingId;
    use crate::ownership::loans::holdings::Holdings;
    use crate::ownership::loans::loan_depth::LoanDepth;
    use crate::ownership::loans::loan_id::LoanId;

    struct RegionFixture;

    impl RegionFixture {
        fn value(index: u32) -> BindingId {
            BindingId::from_index(index)
        }

        fn loans_of(regions: &ValueRegions, index: u32) -> Vec<LoanId> {
            regions.of(Self::value(index)).into_iter().collect()
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
        regions.flow_into(RegionFixture::value(1), &Holdings::from(LoanId::new(0)));
        let resolved = regions.resolved();
        assert_eq!(RegionFixture::loans_of(&resolved, 3), [LoanId::new(0)]);
    }

    #[test]
    fn the_loans_of_holdings_join_their_own_loans_and_their_values_regions() {
        let mut regions = ValueRegions::default();
        regions.flow_into(RegionFixture::value(1), &Holdings::from(LoanId::new(0)));
        let held =
            Holdings::of_value(RegionFixture::value(1)).merged(Holdings::from(LoanId::new(4)));
        let loans: Vec<LoanId> = regions.resolved().of_holdings(&held).into_iter().collect();
        assert_eq!(loans, [LoanId::new(0), LoanId::new(4)]);
    }

    #[test]
    fn a_value_that_holds_itself_deeper_settles_and_keeps_its_loan() {
        let mut regions = ValueRegions::default();
        let value = RegionFixture::value(1);
        regions.flow_into(value, &Holdings::from(LoanId::new(0)));
        regions.flow_into(value, &Holdings::of_value(value).within());
        assert_eq!(
            RegionFixture::loans_of(&regions.resolved(), 1),
            [LoanId::new(0)]
        );
    }

    #[test]
    fn the_deepest_layer_keeps_its_loans_under_any_referent() {
        let mut regions = ValueRegions::default();
        let buried = (0..=LoanDepth::DEEPEST.index())
            .fold(Holdings::from(LoanId::new(0)), |held, _| held.within());
        regions.flow_into(RegionFixture::value(1), &buried);
        let read = (0..=LoanDepth::DEEPEST.index())
            .fold(Holdings::of_value(RegionFixture::value(1)), |held, _| {
                held.referent()
            });
        let loans: Vec<LoanId> = regions.resolved().of_holdings(&read).into_iter().collect();
        assert_eq!(loans, [LoanId::new(0)]);
    }
}
