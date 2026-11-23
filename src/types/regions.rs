use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::BindingId;

#[derive(Default)]
pub struct Regions {
    results: FxHashMap<BindingId, ResultRegion>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResultRegion {
    borrowed: FxHashSet<BindingId>,
    higher_ranked: FxHashSet<BindingId>,
}

impl Regions {
    pub fn record(&mut self, function: BindingId, region: ResultRegion) {
        self.results.insert(function, region);
    }

    pub fn of(&self, function: BindingId) -> Option<&ResultRegion> {
        self.results.get(&function)
    }
}

impl ResultRegion {
    pub fn new(borrowed: FxHashSet<BindingId>, higher_ranked: FxHashSet<BindingId>) -> Self {
        Self {
            borrowed,
            higher_ranked,
        }
    }
}
