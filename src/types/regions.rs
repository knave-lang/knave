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

    pub fn assert_recorded(&self, function: BindingId, region: &ResultRegion) {
        debug_assert!(
            self.of(function).is_none_or(|recorded| recorded == region),
            "the emitter and the ownership stage disagree on what a function's result borrows"
        );
    }
}
