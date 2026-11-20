use super::declarations::Declarations;
use super::open_field_owner::OpenFieldOwner;
use super::ty::{Bound, Type, TypeVar};
use super::unborrowed::Unborrowed;
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
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        let binding = field_traits.bindings().trait_of(field)?;
        if let Some(read) = self.declared_read(owner, binding) {
            return Some(read);
        }
        let open = OpenFieldOwner::new(self.unifier);
        let var = open.var(owner)?;
        Some(match open.bound_read(var, binding) {
            Some(read) => read,
            None => self.new_read(var, binding),
        })
    }

    fn declared_read(&self, owner: &Type, binding: BindingId) -> Option<Type> {
        let Type::Generic(generic) = Unborrowed::of(owner) else {
            return None;
        };
        let bounds = self.unifier.generic_bounds(*generic).iter();
        bounds
            .filter(|bound| bound.binding == binding)
            .find_map(|bound| bound.argument.clone())
    }

    fn new_read(&mut self, var: TypeVar, binding: BindingId) -> Type {
        let read = self.unifier.fresh();
        self.unifier
            .add_bound(var, Bound::with_argument(binding, read.clone()));
        read
    }
}
