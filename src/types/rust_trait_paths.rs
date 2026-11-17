use std::rc::Rc;

use knave_expand::rust_trait_path::RustTraitPath;
use rustc_hash::FxHashMap;

use crate::core::BindingId;

#[derive(Default)]
pub struct RustTraitPaths {
    written: FxHashMap<BindingId, Rc<str>>,
    resolved: FxHashMap<BindingId, RustTraitPath>,
}

impl RustTraitPaths {
    pub fn written(&self, binding: BindingId) -> Option<&Rc<str>> {
        self.written.get(&binding)
    }

    pub(super) fn resolved(&self, binding: BindingId) -> Option<&RustTraitPath> {
        self.resolved.get(&binding)
    }

    pub(super) fn record(&mut self, binding: BindingId, path: &Rc<str>) {
        self.written.insert(binding, Rc::clone(path));
        self.resolved.insert(binding, (**path).into());
    }
}
