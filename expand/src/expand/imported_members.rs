use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::trait_methods::TraitMethods;
use crate::core::{BindingArena, BindingId, BindingKind, Namespace};
use crate::modules::ModuleRecord;

#[derive(Clone, Copy)]
pub(super) struct MemberTables<'e> {
    bindings: &'e BindingArena,
    trait_methods: &'e FxHashMap<BindingId, TraitMethods>,
}

impl<'e> MemberTables<'e> {
    pub(super) fn new(
        bindings: &'e BindingArena,
        trait_methods: &'e FxHashMap<BindingId, TraitMethods>,
    ) -> Self {
        Self {
            bindings,
            trait_methods,
        }
    }
}

pub(super) struct ImportedMembers<'e> {
    target: &'e ModuleRecord,
    bindings: &'e BindingArena,
    trait_methods: &'e FxHashMap<BindingId, TraitMethods>,
}

impl<'e> ImportedMembers<'e> {
    pub(super) fn new(target: &'e ModuleRecord, tables: MemberTables<'e>) -> Self {
        Self {
            target,
            bindings: tables.bindings,
            trait_methods: tables.trait_methods,
        }
    }

    pub(super) fn of(&self, name: &str, type_binding: BindingId) -> Vec<(Rc<str>, BindingId)> {
        match self.bindings.get(type_binding).kind() {
            BindingKind::Enum(_) => self.constructors(name),
            BindingKind::Trait(_) => self.methods(type_binding),
            _ => Vec::new(),
        }
    }

    fn constructors(&self, name: &str) -> Vec<(Rc<str>, BindingId)> {
        let prefix = format!("{name}/");
        self.target
            .exports()
            .entries()
            .filter(|(name, namespace, _)| {
                *namespace == Namespace::Value && name.starts_with(&prefix)
            })
            .map(|(name, _, binding)| (name, binding))
            .collect()
    }

    fn methods(&self, trait_binding: BindingId) -> Vec<(Rc<str>, BindingId)> {
        let Some(trait_methods) = self.trait_methods.get(&trait_binding) else {
            return Vec::new();
        };
        trait_methods
            .methods
            .iter()
            .map(|(method, _)| (Rc::from(self.bindings.get(*method).name()), *method))
            .filter(|(name, method)| self.target.exported(name, Namespace::Value) == Some(*method))
            .collect()
    }
}
