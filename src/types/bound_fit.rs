use super::selection::Selection;
use super::traits::TraitTable;
use super::ty::{Bound, Type, TypeVar};
use super::unifier::Unifier;

pub(super) struct BoundFit<'a> {
    unifier: &'a Unifier,
    traits: &'a TraitTable,
}

impl<'a> BoundFit<'a> {
    pub(super) fn new(unifier: &'a Unifier, traits: &'a TraitTable) -> Self {
        Self { unifier, traits }
    }

    pub(super) fn misses_a_bound(&self, var: TypeVar, value: &Type) -> bool {
        self.unifier.bounds(var).iter().any(|bound| {
            !self.traits.satisfies(bound, value, &|generic, required| {
                self.unifier.generic_bounds(generic).contains(required)
            })
        })
    }

    pub(super) fn passes_through(&self, var: TypeVar, reference: &Type, referent: &Type) -> bool {
        self.misses_a_bound(var, reference) && self.selects_every_bound(var, referent)
    }

    fn selects_every_bound(&self, var: TypeVar, value: &Type) -> bool {
        self.unifier
            .bounds(var)
            .iter()
            .all(|bound| self.selects_bound(bound, value))
    }

    fn selects_bound(&self, bound: &Bound, value: &Type) -> bool {
        match value {
            Type::Generic(generic) => self.unifier.generic_bounds(*generic).contains(bound),
            _ => matches!(
                self.traits.rows(bound.binding).select(bound, value),
                Selection::Implemented { .. }
            ),
        }
    }
}
