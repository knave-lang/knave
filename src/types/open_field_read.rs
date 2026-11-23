use std::rc::Rc;

use super::bound_origins::BoundOrigin;
use super::declarations::Declarations;
use super::inferred_read::InferredRead;
use super::open_read_facts::{OpenReadFacts, OpenedRead};
use super::settled_read::SettledRead;
use super::ty::{Bound, Type, TypeVar};
use super::unifier::Unifier;
use crate::core::{BindingId, FieldLabel};
use crate::source::Span;

pub(super) struct OpenFieldRead<'c> {
    unifier: &'c mut Unifier,
    reads: &'c mut OpenReadFacts,
    declarations: &'c Declarations,
}

impl<'c> OpenFieldRead<'c> {
    pub(super) fn new(
        unifier: &'c mut Unifier,
        reads: &'c mut OpenReadFacts,
        declarations: &'c Declarations,
    ) -> Self {
        Self {
            unifier,
            reads,
            declarations,
        }
    }

    pub(super) fn infer(&mut self, owner: &Type, label: &FieldLabel) -> Option<Type> {
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        let binding = field_traits.bindings().trait_of(&label.name)?;
        let owner = self.open_owner(owner, label)?;
        Some(self.open(owner, binding, label))
    }

    pub(super) fn candidates(&self, label: &FieldLabel) -> Vec<BindingId> {
        match self.pinned_owner(label.span) {
            Some(owner) => vec![owner],
            None => self.declarations.types().structs.with_field(&label.name),
        }
    }

    pub(super) fn request(&mut self, field: &Rc<str>) {
        self.reads.request(field);
    }

    fn open_owner(&self, owner: &Type, label: &FieldLabel) -> Option<TypeVar> {
        match self.unifier.shallow(owner) {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.open_owner(&inner, label),
            Type::Var(var) if self.is_open(var, label) => Some(var),
            _ => None,
        }
    }

    fn is_open(&self, owner: TypeVar, label: &FieldLabel) -> bool {
        let pinned = self.pinned_owner(label.span).is_some();
        self.unifier.literal_kind(owner).is_none() && !pinned
    }

    fn pinned_owner(&self, read: Span) -> Option<BindingId> {
        match self.reads.settled(read)? {
            SettledRead::Owner(owner) => Some(*owner),
            SettledRead::Read(_) => None,
        }
    }

    fn open(&mut self, owner: TypeVar, binding: BindingId, label: &FieldLabel) -> Type {
        let read = label.span;
        let known = (self.unifier.bounds(owner).iter()).find(|bound| bound.binding == binding);
        if let Some(argument) = known.and_then(|bound| bound.argument.clone()) {
            return argument;
        }
        let (argument, origin) = match self.reads.settled(read) {
            Some(SettledRead::Read(settled)) => (settled.clone(), InferredRead::settled(read)),
            _ => (self.unifier.fresh(), InferredRead::open(read)),
        };
        let mut bound = Bound::with_argument(binding, argument.clone());
        bound.origins.add(BoundOrigin::InferredRead(origin));
        self.unifier.add_bound(owner, bound);
        let opened = OpenedRead {
            owner,
            read: argument.clone(),
            label: label.clone(),
        };
        self.reads.open(opened);
        argument
    }
}
