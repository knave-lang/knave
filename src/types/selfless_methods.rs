use rustc_hash::FxHashMap;

use super::selfless_method::SelflessMethod;
use crate::core::BindingId;

#[derive(Default)]
pub(super) struct SelflessMethods {
    methods: FxHashMap<BindingId, SelflessMethod>,
}

impl SelflessMethods {
    pub fn insert(&mut self, method: BindingId, selfless: SelflessMethod) {
        self.methods.insert(method, selfless);
    }

    pub fn get(&self, method: BindingId) -> Option<SelflessMethod> {
        self.methods.get(&method).copied()
    }
}
