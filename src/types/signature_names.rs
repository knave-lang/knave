use rustc_hash::{FxHashMap, FxHashSet};

use super::generic_registry::GenericRegistry;
use super::mentioned_generics::MentionedGenerics;
use super::ty::{GenericId, Scheme, Type};
use crate::core::BindingId;

pub(super) struct SignatureNames<'c> {
    schemes: &'c FxHashMap<BindingId, Scheme>,
    component: &'c FxHashMap<BindingId, Type>,
    generics: &'c GenericRegistry,
}

impl<'c> SignatureNames<'c> {
    pub(super) fn new(
        schemes: &'c FxHashMap<BindingId, Scheme>,
        component: &'c FxHashMap<BindingId, Type>,
        generics: &'c GenericRegistry,
    ) -> Self {
        Self {
            schemes,
            component,
            generics,
        }
    }

    pub(super) fn of(&self, generic: GenericId) -> FxHashSet<&'c str> {
        let signature = self.signature_of(generic).unwrap_or_else(|| vec![generic]);
        (signature.iter())
            .filter_map(|id| self.generics.param(*id))
            .map(|param| param.name.as_ref())
            .collect()
    }

    fn signature_of(&self, generic: GenericId) -> Option<Vec<GenericId>> {
        let scheme = (self.schemes.values())
            .find(|scheme| scheme.params.iter().any(|param| param.id == generic));
        match scheme {
            Some(scheme) => Some(scheme.params.iter().map(|param| param.id).collect()),
            None => (self.component.values())
                .map(|body| self.generics_in(body))
                .find(|found| found.contains(&generic)),
        }
    }

    fn generics_in(&self, body: &Type) -> Vec<GenericId> {
        MentionedGenerics::of(body)
            .with_bound_arguments(|id| self.generics.param(id))
            .into_ids()
    }
}
