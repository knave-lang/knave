use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::copy::CopyRules;
use super::declarations::Declarations;
use super::declared_types::DeclaredTypes;
use super::display::TypeDisplay;
use super::ty::{GenericId, GenericParam};
use super::unification::Unification;
use super::unifier::Unifier;
use crate::core::{BindingArena, BindingId};

#[derive(Clone, Copy)]
pub(super) struct PassingTables<'a> {
    pub(super) declarations: &'a Declarations,
    pub(super) bindings: &'a BindingArena,
    pub(super) generics: &'a FxHashMap<GenericId, GenericParam>,
    pub(super) single_field_readers: &'a FxHashMap<BindingId, Rc<str>>,
    pub(super) copy_trait: Option<BindingId>,
}

impl<'a> PassingTables<'a> {
    pub(super) fn unification<'u>(&self, unifier: &'u mut Unifier) -> Unification<'u>
    where
        'a: 'u,
    {
        Unification::new(unifier, self.display(), self.declared_types())
    }

    pub(super) fn display(&self) -> TypeDisplay<'a> {
        TypeDisplay::new(self.bindings, self.generics)
            .with_projections(self.declarations.traits().table.projections())
    }

    pub(super) fn declared_types(&self) -> DeclaredTypes<'a> {
        DeclaredTypes::new(self.declarations.types())
    }

    pub(super) fn copy_rules(&self) -> CopyRules<'a> {
        CopyRules::new(self.declarations, self.generics)
    }
}
