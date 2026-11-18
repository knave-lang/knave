use std::collections::BTreeSet;

use rustc_hash::FxHashMap;

use super::ty::{Bound, Type};
use crate::core::BindingId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructuralUse {
    pub trait_binding: BindingId,
    pub head: BindingId,
}

#[derive(Clone)]
pub(super) struct ImplBoundUse {
    pub bound: Bound,
    pub implementor: Type,
}

#[derive(Default)]
pub struct StructuralUses {
    by_definition: FxHashMap<BindingId, BTreeSet<StructuralUse>>,
    impl_uses: Vec<ImplBoundUse>,
}

impl StructuralUses {
    pub fn of_definitions<'u>(
        &'u self,
        definitions: impl IntoIterator<Item = &'u BindingId>,
    ) -> BTreeSet<StructuralUse> {
        definitions
            .into_iter()
            .filter_map(|definition| self.by_definition.get(definition))
            .flatten()
            .copied()
            .collect()
    }

    pub(super) fn record_impl_use(&mut self, used: ImplBoundUse) {
        self.impl_uses.push(used);
    }

    pub(super) fn impl_uses(&self) -> &[ImplBoundUse] {
        &self.impl_uses
    }

    pub(super) fn record(&mut self, definitions: &[BindingId], used: StructuralUse) {
        for definition in definitions {
            self.by_definition
                .entry(*definition)
                .or_default()
                .insert(used);
        }
    }
}
