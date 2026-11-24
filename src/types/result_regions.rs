use rustc_hash::FxHashSet;

use super::parameter_role::ParameterRole;
use super::parameterized::Parameterized;
use super::regions::Regions;
use crate::core::BindingId;

pub struct ResultRegions<'r, L> {
    regions: &'r Regions,
    lambda: &'r L,
}

impl<'r, L: Parameterized> ResultRegions<'r, L> {
    pub fn of(regions: &'r Regions, lambda: &'r L) -> Self {
        Self { regions, lambda }
    }

    pub fn kept(&self) -> FxHashSet<BindingId> {
        self.union_of(&[
            ParameterRole::HeldByResult,
            ParameterRole::FlowsThroughSharedArguments,
        ])
    }

    pub fn sharing(&self) -> FxHashSet<BindingId> {
        self.union_of(&[
            ParameterRole::FlowsToResult,
            ParameterRole::StoredValue,
            ParameterRole::StoredTarget,
            ParameterRole::Reassigned,
        ])
    }

    pub fn store_only_targets(&self) -> FxHashSet<BindingId> {
        let flows = self.role(ParameterRole::FlowsToResult);
        self.role(ParameterRole::StoredTarget)
            .difference(&flows)
            .copied()
            .collect()
    }

    pub fn lending_positions(&self) -> Vec<usize> {
        let left_out = self.role(ParameterRole::LeftOutOfResult);
        self.lambda
            .parameter_bindings()
            .enumerate()
            .filter(|(_, binding)| !left_out.contains(binding))
            .map(|(position, _)| position)
            .collect()
    }

    fn union_of(&self, roles: &[ParameterRole]) -> FxHashSet<BindingId> {
        roles.iter().flat_map(|role| self.role(*role)).collect()
    }

    fn role(&self, role: ParameterRole) -> FxHashSet<BindingId> {
        self.regions.of(role, self.lambda)
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::{ParameterRole, Parameterized, Regions, ResultRegions};
    use crate::core::BindingId;

    struct Parameters(Vec<BindingId>);

    impl Parameterized for Parameters {
        fn parameter_bindings(&self) -> impl Iterator<Item = BindingId> + Clone + '_ {
            self.0.iter().copied()
        }
    }

    #[test]
    fn a_parameter_kept_through_a_shared_argument_is_kept_by_the_result() {
        let shared = BindingId::from_index(1);
        let held = BindingId::from_index(2);
        let lambda = Parameters(vec![shared, held, BindingId::from_index(3)]);
        let mut regions = Regions::default();
        regions.record(
            ParameterRole::FlowsThroughSharedArguments,
            &lambda,
            FxHashSet::from_iter([shared]),
        );
        regions.record(
            ParameterRole::HeldByResult,
            &lambda,
            FxHashSet::from_iter([held]),
        );
        assert_eq!(
            ResultRegions::of(&regions, &lambda).kept(),
            FxHashSet::from_iter([shared, held])
        );
    }

    #[test]
    fn a_parameter_the_result_leaves_out_is_not_a_lending_position() {
        let left_out = BindingId::from_index(2);
        let lambda = Parameters(vec![
            BindingId::from_index(1),
            left_out,
            BindingId::from_index(3),
        ]);
        let mut regions = Regions::default();
        regions.record(
            ParameterRole::LeftOutOfResult,
            &lambda,
            FxHashSet::from_iter([left_out]),
        );
        assert_eq!(
            ResultRegions::of(&regions, &lambda).lending_positions(),
            vec![0, 2]
        );
    }
}
