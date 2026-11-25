use std::collections::BTreeMap;

use super::stored_layer::StoredLayer;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorePairs {
    values_by_target: BTreeMap<usize, Vec<(usize, StoredLayer)>>,
}

impl StorePairs {
    pub fn tie(&mut self, value: usize, layer: StoredLayer, target: usize) {
        self.values_by_target
            .entry(target)
            .or_default()
            .push((value, layer));
    }

    pub fn stores(&self) -> impl Iterator<Item = (usize, &[(usize, StoredLayer)])> + '_ {
        self.values_by_target
            .iter()
            .map(|(target, values)| (*target, values.as_slice()))
    }
}
