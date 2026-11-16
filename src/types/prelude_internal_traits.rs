use rustc_hash::FxHashSet;

use crate::core::{BindingId, CoreModule};
use crate::modules::ModuleSources;

#[derive(Default)]
pub struct PreludeInternalTraits {
    bindings: FxHashSet<BindingId>,
}

impl PreludeInternalTraits {
    pub fn record(&mut self, module: &CoreModule, trait_binding: BindingId) {
        if ModuleSources::is_prelude(&module.path) && !module.exports.contains(&trait_binding) {
            self.bindings.insert(trait_binding);
        }
    }

    pub fn contains(&self, trait_binding: BindingId) -> bool {
        self.bindings.contains(&trait_binding)
    }
}
