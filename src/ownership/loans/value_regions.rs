use std::collections::BTreeSet;

use rustc_hash::FxHashMap;

use super::held_value::HeldValue;
use super::holdings::Holdings;
use super::loan_id::LoanId;
use crate::core::BindingId;

#[derive(Default)]
pub struct ValueRegions {
    loans: FxHashMap<BindingId, Vec<BTreeSet<LoanId>>>,
    sources: FxHashMap<BindingId, BTreeSet<HeldValue>>,
}

impl ValueRegions {
    const DEEPEST_LAYER: usize = 7;

    pub fn flow_into(&mut self, binding: BindingId, held: &Holdings) {
        for (depth, loan) in held.loans() {
            self.layer_mut(binding, depth).insert(loan);
        }
        self.sources
            .entry(binding)
            .or_default()
            .extend(held.values());
    }

    pub fn resolved(mut self) -> Self {
        while self.grow() {}
        self
    }

    pub fn of(&self, binding: BindingId) -> BTreeSet<LoanId> {
        self.layers_of(binding)
            .flat_map(|(_, loans)| loans.iter().copied())
            .collect()
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        held.loans()
            .map(|(_, loan)| loan)
            .chain(held.values().flat_map(|value| {
                self.placed_layers(value)
                    .flat_map(|(_, loans)| loans.iter().copied())
            }))
            .collect()
    }

    fn layers_of(&self, binding: BindingId) -> impl Iterator<Item = (usize, &BTreeSet<LoanId>)> {
        self.loans.get(&binding).into_iter().flatten().enumerate()
    }

    fn placed_layers(
        &self,
        value: HeldValue,
    ) -> impl Iterator<Item = (usize, &BTreeSet<LoanId>)> + '_ {
        self.layers_of(value.binding())
            .filter_map(move |(layer, loans)| Some((self.placed(value, layer)?, loans)))
    }

    fn placed(&self, value: HeldValue, layer: usize) -> Option<usize> {
        match layer {
            Self::DEEPEST_LAYER => Some(Self::DEEPEST_LAYER),
            _ => value.placed(layer),
        }
    }

    fn layer_mut(&mut self, binding: BindingId, depth: usize) -> &mut BTreeSet<LoanId> {
        let depth = depth.min(Self::DEEPEST_LAYER);
        let layers = self.loans.entry(binding).or_default();
        if layers.len() <= depth {
            layers.resize_with(depth + 1, BTreeSet::new);
        }
        &mut layers[depth]
    }

    fn grow(&mut self) -> bool {
        let regions: &Self = self;
        let additions: Vec<(BindingId, usize, LoanId)> = regions
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
            self.layer_mut(binding, depth).insert(loan);
        }
        grew
    }

    fn missing_from(
        &self,
        binding: BindingId,
        source: HeldValue,
    ) -> impl Iterator<Item = (BindingId, usize, LoanId)> + '_ {
        self.placed_layers(source).flat_map(move |(depth, loans)| {
            let depth = depth.min(Self::DEEPEST_LAYER);
            loans
                .iter()
                .copied()
                .filter(move |loan| !self.holds(binding, depth, *loan))
                .map(move |loan| (binding, depth, loan))
        })
    }

    fn holds(&self, binding: BindingId, depth: usize, loan: LoanId) -> bool {
        self.loans
            .get(&binding)
            .and_then(|layers| layers.get(depth))
            .is_some_and(|loans| loans.contains(&loan))
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

    #[test]
    fn a_value_that_holds_itself_deeper_settles_and_keeps_its_loan() {
        let mut regions = ValueRegions::default();
        let value = RegionFixture::value(1);
        regions.flow_into(value, &Holdings::lent(LoanId::new(0)));
        regions.flow_into(value, &Holdings::of_value(value).within());
        assert_eq!(
            RegionFixture::loans_of(&regions.resolved(), 1),
            [LoanId::new(0)]
        );
    }

    #[test]
    fn the_deepest_layer_keeps_its_loans_under_any_referent() {
        let mut regions = ValueRegions::default();
        let buried = (0..=ValueRegions::DEEPEST_LAYER)
            .fold(Holdings::lent(LoanId::new(0)), |held, _| held.within());
        regions.flow_into(RegionFixture::value(1), &buried);
        let read = (0..=ValueRegions::DEEPEST_LAYER)
            .fold(Holdings::of_value(RegionFixture::value(1)), |held, _| {
                held.referent()
            });
        let loans: Vec<LoanId> = regions.resolved().of_holdings(&read).into_iter().collect();
        assert_eq!(loans, [LoanId::new(0)]);
    }
}
