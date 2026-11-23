use rustc_hash::FxHashSet;

use super::callee_parameters::CalleeParameters;
use crate::core::BindingId;
use crate::types::TypedLambda;

pub struct LambdaFlows {
    pub flows: FxHashSet<BindingId>,
    pub higher_ranked: FxHashSet<BindingId>,
    pub(super) earliest_reached: Option<usize>,
}

impl LambdaFlows {
    pub(super) fn parameters(&self, lambda: &TypedLambda) -> CalleeParameters {
        CalleeParameters::new(
            Self::marked(lambda, &self.flows),
            Self::marked(lambda, &self.higher_ranked),
        )
    }

    fn marked(lambda: &TypedLambda, bindings: &FxHashSet<BindingId>) -> Vec<bool> {
        lambda
            .params
            .iter()
            .map(|param| bindings.contains(&param.binding))
            .collect()
    }
}

pub struct FunctionFlows {
    pub flows: LambdaFlows,
    pub reassigned_parameters: FxHashSet<BindingId>,
}
