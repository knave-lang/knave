use rustc_hash::FxHashSet;

use super::stored_arguments::Store;
use crate::core::BindingId;
use crate::types::{StoredFlows, TypedLambda};

#[derive(Default)]
pub struct DefinitionStores {
    pub flows: StoredFlows,
    pub(super) positions: Vec<Store>,
}

impl DefinitionStores {
    pub(super) fn new(flows: StoredFlows, lambda: &TypedLambda) -> Self {
        let positions = Self::positions(&flows, lambda);
        Self { flows, positions }
    }

    fn positions(flows: &StoredFlows, lambda: &TypedLambda) -> Vec<Store> {
        let values = Self::marked(lambda, &flows.values);
        Self::marked(lambda, &flows.targets)
            .into_iter()
            .flat_map(|target| {
                values
                    .iter()
                    .filter(move |value| **value != target)
                    .map(move |value| Store {
                        target,
                        value: *value,
                    })
            })
            .collect()
    }

    fn marked(lambda: &TypedLambda, bindings: &FxHashSet<BindingId>) -> Vec<usize> {
        lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, param)| bindings.contains(&param.binding))
            .map(|(position, _)| position)
            .collect()
    }
}
