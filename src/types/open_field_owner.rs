use super::ty::{Type, TypeVar};
use super::unifier::Unifier;
use crate::core::BindingId;

pub(super) struct OpenFieldOwner<'u> {
    unifier: &'u Unifier,
}

impl<'u> OpenFieldOwner<'u> {
    pub(super) fn new(unifier: &'u Unifier) -> Self {
        Self { unifier }
    }

    pub(super) fn var(&self, owner: &Type) -> Option<TypeVar> {
        match self.unborrowed(owner) {
            Type::Var(var) if self.unifier.literal_kind(var).is_none() => Some(var),
            _ => None,
        }
    }

    pub(super) fn bound_read(&self, var: TypeVar, field_trait: BindingId) -> Option<Type> {
        let mut bounds = self.unifier.bounds(var).iter();
        let bound = bounds.find(|bound| bound.binding == field_trait)?;
        bound.argument.clone()
    }

    fn unborrowed(&self, owner: &Type) -> Type {
        match self.unifier.shallow(owner) {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.unborrowed(&inner),
            resolved => resolved,
        }
    }
}
