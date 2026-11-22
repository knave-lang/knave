use rustc_hash::FxHashMap;

use crate::core::{BindingId, ImplMethod, ItemKind, TypeExpr, TypeExprKind};
use crate::expand::Program;

pub(super) struct WrittenImpls<'p> {
    entries: Vec<ImplEntry<'p>>,
    methods_of_trait: FxHashMap<BindingId, Vec<BindingId>>,
    methods_of_impl: FxHashMap<(BindingId, BindingId), Vec<BindingId>>,
    trait_of_method: FxHashMap<BindingId, BindingId>,
}

pub(super) struct ImplEntry<'p> {
    pub(super) trait_binding: BindingId,
    pub(super) implementor: Option<BindingId>,
    pub(super) methods: &'p [ImplMethod],
}

impl<'p> WrittenImpls<'p> {
    pub(super) fn of(program: &'p Program) -> Self {
        let entries: Vec<ImplEntry<'p>> = Self::entries_of(program).collect();
        Self {
            methods_of_trait: Self::methods_by_trait(&entries),
            methods_of_impl: Self::methods_by_impl(&entries),
            trait_of_method: Self::trait_of_each_method(&entries),
            entries,
        }
    }

    pub(super) fn entries(&self) -> &[ImplEntry<'p>] {
        &self.entries
    }

    pub(super) fn methods_of_trait(&self, trait_binding: BindingId) -> Option<&[BindingId]> {
        self.methods_of_trait.get(&trait_binding).map(Vec::as_slice)
    }

    pub(super) fn methods_of_impl(
        &self,
        trait_binding: BindingId,
        head: BindingId,
    ) -> &[BindingId] {
        self.methods_of_impl
            .get(&(trait_binding, head))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn trait_of(&self, method: BindingId) -> Option<BindingId> {
        self.trait_of_method.get(&method).copied()
    }

    fn entries_of(program: &'p Program) -> impl Iterator<Item = ImplEntry<'p>> + use<'p> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Impl(trait_binding, implementor, _, methods) => Some(ImplEntry {
                    trait_binding: *trait_binding,
                    implementor: Self::head(implementor),
                    methods: methods.as_slice(),
                }),
                _ => None,
            })
    }

    fn head(implementor: &TypeExpr) -> Option<BindingId> {
        match implementor.kind() {
            TypeExprKind::Named(binding, _) => Some(*binding),
            _ => None,
        }
    }

    fn methods_by_trait(entries: &[ImplEntry<'_>]) -> FxHashMap<BindingId, Vec<BindingId>> {
        let mut by_trait: FxHashMap<BindingId, Vec<BindingId>> = FxHashMap::default();
        for entry in entries {
            by_trait
                .entry(entry.trait_binding)
                .or_default()
                .extend(entry.methods.iter().map(|method| method.binding));
        }
        by_trait
    }

    fn methods_by_impl(
        entries: &[ImplEntry<'_>],
    ) -> FxHashMap<(BindingId, BindingId), Vec<BindingId>> {
        let mut by_impl: FxHashMap<(BindingId, BindingId), Vec<BindingId>> = FxHashMap::default();
        for entry in entries {
            let Some(implementor) = entry.implementor else {
                continue;
            };
            by_impl
                .entry((entry.trait_binding, implementor))
                .or_default()
                .extend(entry.methods.iter().map(|method| method.binding));
        }
        by_impl
    }

    fn trait_of_each_method(entries: &[ImplEntry<'_>]) -> FxHashMap<BindingId, BindingId> {
        entries
            .iter()
            .flat_map(|entry| {
                entry
                    .methods
                    .iter()
                    .map(|method| (method.binding, entry.trait_binding))
            })
            .collect()
    }
}
