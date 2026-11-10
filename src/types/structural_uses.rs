use std::collections::BTreeSet;

use rustc_hash::FxHashMap;

use super::ty::{Bound, Type};
use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructuralUse {
    pub trait_binding: BindingId,
    pub head: BindingId,
}

#[derive(Clone)]
pub(super) struct ImplBoundUse {
    pub definitions: Vec<BindingId>,
    pub bound: Bound,
    pub implementor: Type,
    pub span: Span,
}

#[derive(Default)]
pub struct StructuralUses {
    by_definition: FxHashMap<BindingId, BTreeSet<StructuralUse>>,
    through_impls: Vec<ImplBoundUse>,
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

    pub(super) fn defer_until_impls(&mut self, pending: ImplBoundUse) {
        self.through_impls.push(pending);
    }

    pub(super) fn deferred_until_impls(&self) -> &[ImplBoundUse] {
        &self.through_impls
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
