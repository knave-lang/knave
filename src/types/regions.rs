use rustc_hash::FxHashMap;

use super::ResultRegion;
use crate::core::BindingId;

#[derive(Default)]
pub struct Regions {
    results: FxHashMap<BindingId, ResultRegion>,
}

impl Regions {
    pub fn record(&mut self, function: BindingId, region: ResultRegion) {
        self.results.insert(function, region);
    }

    pub fn of(&self, function: BindingId) -> Option<&ResultRegion> {
        self.results.get(&function)
    }
}
