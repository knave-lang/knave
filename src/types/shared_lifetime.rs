use rustc_hash::FxHashSet;

use super::parameter_role::ParameterRole;
use super::parameterized::Parameterized;
use super::regions::Regions;
use crate::core::BindingId;

pub struct SharedLifetime<'r, L> {
    regions: &'r Regions,
    lambda: &'r L,
}

impl<'r, L: Parameterized> SharedLifetime<'r, L> {
    pub fn of(regions: &'r Regions, lambda: &'r L) -> Self {
        Self { regions, lambda }
    }

    pub fn forced_by_stores(&self) -> bool {
        let reassigned = self.role(ParameterRole::Reassigned);
        !self
            .regions
            .stored(self.lambda)
            .with_values(reassigned)
            .is_empty()
    }

    pub fn sharing(&self) -> FxHashSet<BindingId> {
        [
            ParameterRole::FlowsToResult,
            ParameterRole::StoredValue,
            ParameterRole::StoredTarget,
            ParameterRole::Reassigned,
        ]
        .into_iter()
        .flat_map(|role| self.role(role))
        .collect()
    }

    pub fn store_only_targets(&self) -> FxHashSet<BindingId> {
        let flows = self.role(ParameterRole::FlowsToResult);
        self.role(ParameterRole::StoredTarget)
            .difference(&flows)
            .copied()
            .collect()
    }

    fn role(&self, role: ParameterRole) -> FxHashSet<BindingId> {
        self.regions.of(role, self.lambda)
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::{ParameterRole, Parameterized, Regions, SharedLifetime};
    use crate::core::BindingId;

    struct Parameters(Vec<BindingId>);

    impl Parameterized for Parameters {
        fn parameter_bindings(&self) -> impl Iterator<Item = BindingId> + Clone + '_ {
            self.0.iter().copied()
        }
    }

    struct LifetimeFixture {
        lambda: Parameters,
        regions: Regions,
    }

    impl LifetimeFixture {
        const ROLES: [ParameterRole; 5] = [
            ParameterRole::FlowsToResult,
            ParameterRole::StoredValue,
            ParameterRole::StoredTarget,
            ParameterRole::Reassigned,
            ParameterRole::HeldByResult,
        ];

        fn with(recorded: &[(ParameterRole, usize)]) -> Self {
            let lambda = Parameters((0..3).map(BindingId::from_index).collect());
            let mut regions = Regions::default();
            for role in Self::ROLES {
                let parameters = recorded
                    .iter()
                    .filter(|(recorded_role, _)| *recorded_role == role)
                    .map(|(_, position)| lambda.0[*position])
                    .collect();
                regions.record(role, &lambda, parameters);
            }
            Self { lambda, regions }
        }

        fn lifetime(&self) -> SharedLifetime<'_, Parameters> {
            SharedLifetime::of(&self.regions, &self.lambda)
        }

        fn bindings(&self, positions: &[usize]) -> FxHashSet<BindingId> {
            positions
                .iter()
                .map(|position| self.lambda.0[*position])
                .collect()
        }
    }

    #[test]
    fn a_stored_value_forces_the_shared_lifetime() {
        let fixture = LifetimeFixture::with(&[(ParameterRole::StoredValue, 0)]);
        assert!(fixture.lifetime().forced_by_stores());
    }

    #[test]
    fn a_reassigned_parameter_forces_the_shared_lifetime() {
        let fixture = LifetimeFixture::with(&[(ParameterRole::Reassigned, 1)]);
        assert!(fixture.lifetime().forced_by_stores());
    }

    #[test]
    fn a_parameter_that_only_flows_to_the_result_forces_nothing() {
        let fixture = LifetimeFixture::with(&[(ParameterRole::FlowsToResult, 0)]);
        assert!(!fixture.lifetime().forced_by_stores());
    }

    #[test]
    fn every_flowing_stored_or_reassigned_parameter_shares_the_lifetime() {
        let fixture = LifetimeFixture::with(&[
            (ParameterRole::FlowsToResult, 0),
            (ParameterRole::StoredTarget, 1),
            (ParameterRole::HeldByResult, 2),
        ]);
        assert_eq!(fixture.lifetime().sharing(), fixture.bindings(&[0, 1]));
    }

    #[test]
    fn a_store_target_that_also_flows_is_not_store_only() {
        let fixture = LifetimeFixture::with(&[
            (ParameterRole::StoredTarget, 0),
            (ParameterRole::StoredTarget, 1),
            (ParameterRole::FlowsToResult, 1),
        ]);
        assert_eq!(
            fixture.lifetime().store_only_targets(),
            fixture.bindings(&[0])
        );
    }
}
