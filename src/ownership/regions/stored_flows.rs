use std::borrow::Cow;

use rustc_hash::FxHashSet;

use super::stored_arguments::Store;
use crate::core::BindingId;
use crate::types::TypedLambda;

#[derive(Default)]
pub struct DefinitionStores {
    pub flows: StoredFlows,
    pub(super) positions: Vec<Store>,
}

#[derive(Clone, Default)]
pub struct StoredFlows {
    pub values: FxHashSet<BindingId>,
    pub targets: FxHashSet<BindingId>,
}

impl StoredFlows {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn with_values(&self, values: FxHashSet<BindingId>) -> Cow<'_, StoredFlows> {
        if values.is_empty() {
            return Cow::Borrowed(self);
        }
        Cow::Owned(StoredFlows {
            values: self.values.union(&values).copied().collect(),
            targets: self.targets.clone(),
        })
    }

    pub(super) fn extend(&mut self, other: StoredFlows) {
        self.values.extend(other.values);
        self.targets.extend(other.targets);
    }

    pub(super) fn positions(&self, lambda: &TypedLambda) -> Vec<Store> {
        let values = Self::marked(lambda, &self.values);
        Self::marked(lambda, &self.targets)
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
