use super::traits::TraitTable;
use super::ty::{Bound, Type};

pub(super) struct PlainBound<'t> {
    traits: &'t TraitTable,
}

impl<'t> PlainBound<'t> {
    pub(super) fn new(traits: &'t TraitTable) -> Self {
        Self { traits }
    }

    pub(super) fn is_met_by(&self, bound: &Bound, read: &Type) -> bool {
        self.traits.satisfies(bound, read, &|_, _| false)
    }
}
