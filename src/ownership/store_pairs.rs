use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorePairs {
    values_by_target: BTreeMap<usize, Vec<usize>>,
}

impl StorePairs {
    pub fn tie(&mut self, value: usize, target: usize) {
        self.values_by_target.entry(target).or_default().push(value);
    }

    pub fn stores(&self) -> impl Iterator<Item = (usize, &[usize])> + '_ {
        self.values_by_target
            .iter()
            .map(|(target, values)| (*target, values.as_slice()))
    }
}
