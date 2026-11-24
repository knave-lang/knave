use rustc_hash::{FxHashMap, FxHashSet};

use super::parameter_role::ParameterRole;
use super::parameterized::Parameterized;
use super::result_regions::ResultRegions;
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

    pub fn of_result<'r, L: Parameterized>(&'r self, lambda: &'r L) -> ResultRegions<'r, L> {
        ResultRegions::new(self, lambda)
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
