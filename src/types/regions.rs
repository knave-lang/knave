use rustc_hash::{FxHashMap, FxHashSet};

use super::parameter_role::ParameterRole;
use super::parameterized::Parameterized;
use super::stored_flows::StoredFlows;
use crate::core::BindingId;

#[derive(Default)]
pub struct Regions {
    parameters: FxHashMap<ParameterRole, FxHashSet<BindingId>>,
    #[cfg(debug_assertions)]
    analysed: FxHashMap<ParameterRole, FxHashSet<BindingId>>,
}

impl Regions {
    pub fn record(
        &mut self,
        role: ParameterRole,
        lambda: &impl Parameterized,
        parameters: FxHashSet<BindingId>,
    ) {
        #[cfg(debug_assertions)]
        self.analysed
            .entry(role)
            .or_default()
            .extend(lambda.parameter_bindings());
        self.parameters.entry(role).or_default().extend(parameters);
    }

    pub fn of(&self, role: ParameterRole, lambda: &impl Parameterized) -> FxHashSet<BindingId> {
        self.assert_analysed(role, lambda);
        let Some(recorded) = self.parameters.get(&role) else {
            return FxHashSet::default();
        };
        lambda
            .parameter_bindings()
            .filter(|binding| recorded.contains(binding))
            .collect()
    }

    pub fn kept_by_result(&self, lambda: &impl Parameterized) -> FxHashSet<BindingId> {
        [
            ParameterRole::HeldByResult,
            ParameterRole::FlowsThroughSharedArguments,
        ]
        .into_iter()
        .flat_map(|role| self.of(role, lambda))
        .collect()
    }

    pub fn lending_positions(&self, lambda: &impl Parameterized) -> Vec<usize> {
        let left_out = self.of(ParameterRole::LeftOutOfResult, lambda);
        lambda
            .parameter_bindings()
            .enumerate()
            .filter(|(_, binding)| !left_out.contains(binding))
            .map(|(position, _)| position)
            .collect()
    }

    pub fn stored(&self, lambda: &impl Parameterized) -> StoredFlows {
        StoredFlows {
            values: self.of(ParameterRole::StoredValue, lambda),
            targets: self.of(ParameterRole::StoredTarget, lambda),
        }
    }

    #[cfg(debug_assertions)]
    fn assert_analysed(&self, role: ParameterRole, lambda: &impl Parameterized) {
        let analysed = self.analysed.get(&role);
        assert!(
            lambda
                .parameter_bindings()
                .all(|parameter| analysed.is_some_and(|analysed| analysed.contains(&parameter))),
            "the emitter reads a region the ownership stage did not record"
        );
    }

    #[cfg(not(debug_assertions))]
    fn assert_analysed(&self, _role: ParameterRole, _lambda: &impl Parameterized) {}
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::{ParameterRole, Parameterized, Regions};
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
            regions.kept_by_result(&lambda),
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
        assert_eq!(regions.lending_positions(&lambda), vec![0, 2]);
    }
}
