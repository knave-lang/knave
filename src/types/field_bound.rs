use std::rc::Rc;

use super::bounded_owner_problems::BoundedOwnerProblems;
use super::declarations::Declarations;
use super::problems::TypeProblems;
use super::ty::{GenericId, Type};
use super::unifier::Unifier;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct FieldBound<'c> {
    unifier: &'c Unifier,
    declarations: &'c Declarations,
}

impl<'c> FieldBound<'c> {
    pub(super) fn new(unifier: &'c Unifier, declarations: &'c Declarations) -> Self {
        Self {
            unifier,
            declarations,
        }
    }

    pub(super) fn read(&self, owner: &Type, field: &str) -> Option<Type> {
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        let binding = field_traits.bindings().trait_of(field)?;
        let generic = self.generic_owner(owner)?;
        let bounds = self.unifier.generic_bounds(generic).iter();
        bounds
            .filter(|bound| bound.binding == binding)
            .find_map(|bound| bound.argument.clone())
    }

    pub(super) fn unbound_read(
        &self,
        shown: &str,
        owner: &Type,
        field: &str,
        span: Span,
    ) -> Diagnostic {
        match self.bound_fields(owner) {
            Some(fields) => BoundedOwnerProblems::new(shown, &fields).unbound_read(field, span),
            None => TypeProblems::no_fields(shown, span),
        }
    }

    pub(super) fn unbound_update(&self, shown: &str, owner: &Type, span: Span) -> Diagnostic {
        match self.bound_fields(owner) {
            Some(fields) => BoundedOwnerProblems::new(shown, &fields).update(span),
            None => TypeProblems::no_fields(shown, span),
        }
    }

    fn bound_fields(&self, owner: &Type) -> Option<Vec<Rc<str>>> {
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        let generic = self.generic_owner(owner)?;
        let fields: Vec<Rc<str>> = (self.unifier.generic_bounds(generic).iter())
            .filter_map(|bound| field_traits.bindings().field_of(bound.binding).cloned())
            .collect();
        (!fields.is_empty()).then_some(fields)
    }

    fn generic_owner(&self, owner: &Type) -> Option<GenericId> {
        match self.unifier.shallow(owner) {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.generic_owner(&inner),
            Type::Generic(generic) => Some(generic),
            _ => None,
        }
    }
}
