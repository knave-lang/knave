use super::declarations::Declarations;
use super::ty::{Bound, Type, TypeVar};
use super::unifier::Unifier;
use crate::core::BindingId;

pub(super) struct FieldBound<'c> {
    unifier: &'c mut Unifier,
    declarations: &'c Declarations,
}

impl<'c> FieldBound<'c> {
    pub(super) fn new(unifier: &'c mut Unifier, declarations: &'c Declarations) -> Self {
        Self {
            unifier,
            declarations,
        }
    }

    pub(super) fn read(mut self, owner: &Type, field: &str) -> Option<Type> {
        if let Some(read) = self.declared_read(owner, field) {
            return Some(read);
        }
        let var = self.open_owner(owner)?;
        let shared = self.declarations.types().structs.with_field(field).len() > 1;
        let binding = (self
            .declarations
            .traits()
            .rust_paths
            .field_traits
            .trait_of(field))
        .filter(|_| shared)?;
        Some(match self.known_read(var, binding) {
            Some(read) => read,
            None => self.new_read(var, binding),
        })
    }

    fn declared_read(&self, owner: &Type, field: &str) -> Option<Type> {
        let Type::Generic(generic) = Self::without_borrows(owner) else {
            return None;
        };
        let binding = self
            .declarations
            .traits()
            .rust_paths
            .field_traits
            .trait_of(field)?;
        let bounds = self.unifier.generic_bounds(*generic).iter();
        bounds
            .filter(|bound| bound.binding == binding)
            .find_map(|bound| bound.argument.clone())
    }

    fn without_borrows(owner: &Type) -> &Type {
        match owner {
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::without_borrows(inner),
            _ => owner,
        }
    }

    fn open_owner(&self, owner: &Type) -> Option<TypeVar> {
        let mut owner = self.unifier.shallow(owner);
        while let Type::Ref(_, inner) | Type::Mut(_, inner) = owner {
            owner = self.unifier.shallow(&inner);
        }
        match owner {
            Type::Var(var) if self.unifier.literal_kind(var).is_none() => Some(var),
            _ => None,
        }
    }

    fn known_read(&self, var: TypeVar, binding: BindingId) -> Option<Type> {
        self.unifier
            .bounds(var)
            .iter()
            .find(|bound| bound.binding == binding)
            .and_then(|bound| bound.argument.clone())
    }

    fn new_read(&mut self, var: TypeVar, binding: BindingId) -> Type {
        let read = self.unifier.fresh();
        self.unifier
            .add_bound(var, Bound::with_argument(binding, read.clone()));
        read
    }
}
