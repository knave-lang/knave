use rustc_hash::FxHashMap;

use super::generic_registry::GenericRegistry;
use super::implied_bounds::ImpliedBounds;
use super::ty::{GenericId, Scheme, Type};
use super::unifier::Unifier;
use crate::source::Span;

pub(super) struct Instantiation<'a> {
    generics: &'a GenericRegistry,
    implied: &'a ImpliedBounds,
    unifier: &'a mut Unifier,
}

impl<'a> Instantiation<'a> {
    pub(super) fn new(
        generics: &'a GenericRegistry,
        implied: &'a ImpliedBounds,
        unifier: &'a mut Unifier,
    ) -> Self {
        Self {
            generics,
            implied,
            unifier,
        }
    }

    pub(super) fn instantiate(&mut self, scheme: &Scheme, span: Span) -> Type {
        let fresh: Vec<Type> = scheme
            .params
            .iter()
            .map(|param| self.fresh_instance(param.id))
            .collect();
        let replacements: FxHashMap<GenericId, Type> = scheme
            .params
            .iter()
            .zip(&fresh)
            .map(|(param, value)| (param.id, value.clone()))
            .collect();
        for (param, value) in scheme.params.iter().zip(&fresh) {
            if let Type::Var(var) = value {
                let bounds = param
                    .bounds
                    .iter()
                    .chain(self.implied.of(param.id))
                    .map(|bound| bound.substituted(&replacements))
                    .collect();
                self.unifier.set_bounds(*var, bounds, span);
            }
        }
        scheme.body.substitute(&replacements)
    }

    fn fresh_instance(&mut self, generic: GenericId) -> Type {
        match self.generics.literal(generic) {
            Some(literal) => self.unifier.fresh_literal(literal),
            None => self.unifier.fresh(),
        }
    }
}
