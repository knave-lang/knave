use rustc_hash::FxHashMap;

use super::trait_method_info::TraitMethodInfo;
use crate::core::BindingId;

#[derive(Default)]
pub struct TraitMethods {
    declared: FxHashMap<BindingId, TraitMethodInfo>,
    defaults: FxHashMap<BindingId, BindingId>,
}

impl TraitMethods {
    pub fn get(&self, method: &BindingId) -> Option<&TraitMethodInfo> {
        self.declared.get(method)
    }

    pub fn method_bindings(&self) -> impl Iterator<Item = BindingId> {
        self.declared.keys().copied()
    }

    pub fn defines_any(&self, trait_binding: BindingId) -> bool {
        (self.declared.values()).any(|info| info.trait_binding == trait_binding)
    }

    pub fn trait_defining(&self, method: BindingId) -> Option<BindingId> {
        let declared = self.declared.get(&method).map(|info| info.trait_binding);
        declared.or_else(|| self.defaults.get(&method).copied())
    }

    pub(super) fn record_method(&mut self, method: BindingId, info: TraitMethodInfo) {
        self.declared.insert(method, info);
    }

    pub(super) fn record_default(&mut self, default: BindingId, trait_binding: BindingId) {
        self.defaults.insert(default, trait_binding);
    }
}
