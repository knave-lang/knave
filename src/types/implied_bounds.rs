use rustc_hash::{FxHashMap, FxHashSet};

use super::ty::{Bound, GenericId, GenericParam};

#[derive(Default)]
pub(super) struct ImpliedBounds {
    impl_generics: FxHashSet<GenericId>,
    bounds: FxHashMap<GenericId, Vec<Bound>>,
}

impl ImpliedBounds {
    pub(super) fn mark_impl_generics(&mut self, params: &[GenericParam]) {
        self.impl_generics
            .extend(params.iter().map(|param| param.id));
    }

    pub(super) fn is_impl_generic(&self, generic: GenericId) -> bool {
        self.impl_generics.contains(&generic)
    }

    pub(super) fn record(&mut self, generic: GenericId, bound: Bound) {
        let implied = self.bounds.entry(generic).or_default();
        if !implied.contains(&bound) {
            implied.push(bound);
        }
    }

    pub(super) fn of(&self, generic: GenericId) -> impl Iterator<Item = &Bound> {
        self.bounds.get(&generic).into_iter().flatten()
    }

    pub(super) fn into_map(self) -> FxHashMap<GenericId, Vec<Bound>> {
        self.bounds
    }
}
