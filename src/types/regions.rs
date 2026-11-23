use rustc_hash::{FxHashMap, FxHashSet};

use super::parameter_role::ParameterRole;
use super::stored_flows::StoredFlows;
use crate::core::BindingId;

#[derive(Default)]
pub struct Regions {
    parameters: FxHashMap<ParameterRole, FxHashSet<BindingId>>,
}

impl Regions {
    pub fn record(&mut self, role: ParameterRole, parameters: FxHashSet<BindingId>) {
        self.parameters.entry(role).or_default().extend(parameters);
    }

    pub fn of(
        &self,
        role: ParameterRole,
        parameters: impl Iterator<Item = BindingId>,
    ) -> FxHashSet<BindingId> {
        let Some(recorded) = self.parameters.get(&role) else {
            return FxHashSet::default();
        };
        parameters
            .filter(|binding| recorded.contains(binding))
            .collect()
    }

    pub fn stored(&self, parameters: impl Iterator<Item = BindingId> + Clone) -> StoredFlows {
        StoredFlows {
            values: self.of(ParameterRole::StoredValue, parameters.clone()),
            targets: self.of(ParameterRole::StoredTarget, parameters),
        }
    }
}
