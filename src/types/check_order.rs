use std::cmp::Reverse;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, ImplMethod, ItemKind, References, TypeExpr};
use crate::expand::Program;

use super::type_depths::TypeDepths;

pub(super) struct CheckOrder<'a> {
    references: &'a FxHashMap<BindingId, References>,
    impl_methods_of_trait: FxHashMap<BindingId, Vec<BindingId>>,
    impl_method_depths: FxHashMap<BindingId, usize>,
    visited: FxHashSet<BindingId>,
    order: Vec<BindingId>,
}

struct ImplEntry<'p> {
    trait_binding: BindingId,
    depth: usize,
    methods: &'p [ImplMethod],
}

enum Step {
    Enter(BindingId),
    Leave(BindingId),
}

impl<'a> CheckOrder<'a> {
    pub(super) fn of(
        program: &Program,
        references: &'a FxHashMap<BindingId, References>,
        definitions: impl Iterator<Item = BindingId>,
    ) -> Vec<BindingId> {
        let impls = Self::impls_innermost_first(program);
        let mut walk = Self {
            references,
            impl_methods_of_trait: Self::impl_methods_by_trait(&impls),
            impl_method_depths: Self::impl_method_depths(&impls),
            visited: FxHashSet::default(),
            order: Vec::with_capacity(references.len()),
        };
        let (mut methods, others): (Vec<BindingId>, Vec<BindingId>) =
            definitions.partition(|binding| walk.impl_method_depths.contains_key(binding));
        methods.sort_by_key(|method| Reverse(walk.impl_method_depths[method]));
        for method in methods {
            walk.visit(method);
        }
        walk.order.extend(others);
        walk.order
    }

    #[cfg(test)]
    pub(super) fn impl_methods<'p>(
        program: &'p Program,
    ) -> impl Iterator<Item = BindingId> + use<'p> {
        Self::impls(program)
            .flat_map(|(_, _, methods)| methods)
            .map(|method| method.binding)
    }

    fn impls<'p>(
        program: &'p Program,
    ) -> impl Iterator<Item = (BindingId, &'p TypeExpr, &'p [ImplMethod])> + use<'p> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Impl(trait_binding, implementor, _, methods) => {
                    Some((*trait_binding, implementor, methods.as_slice()))
                }
                _ => None,
            })
    }

    fn impls_innermost_first(program: &Program) -> Vec<ImplEntry<'_>> {
        let mut depths = TypeDepths::of(program);
        let mut impls: Vec<ImplEntry<'_>> = Self::impls(program)
            .map(|(trait_binding, implementor, methods)| ImplEntry {
                trait_binding,
                depth: depths.of_type(implementor),
                methods,
            })
            .collect();
        impls.sort_by_key(|entry| entry.depth);
        impls
    }

    fn impl_methods_by_trait(impls: &[ImplEntry<'_>]) -> FxHashMap<BindingId, Vec<BindingId>> {
        let mut by_trait: FxHashMap<BindingId, Vec<BindingId>> = FxHashMap::default();
        for entry in impls {
            by_trait
                .entry(entry.trait_binding)
                .or_default()
                .extend(entry.methods.iter().map(|method| method.binding));
        }
        by_trait
    }

    fn impl_method_depths(impls: &[ImplEntry<'_>]) -> FxHashMap<BindingId, usize> {
        impls
            .iter()
            .flat_map(|entry| {
                entry
                    .methods
                    .iter()
                    .map(|method| (method.binding, entry.depth))
            })
            .collect()
    }

    fn visit(&mut self, root: BindingId) {
        let mut steps = vec![Step::Enter(root)];
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(binding) if self.visited.insert(binding) => {
                    steps.push(Step::Leave(binding));
                    steps.extend(self.successors(binding).into_iter().rev().map(Step::Enter));
                }
                Step::Enter(_) => {}
                Step::Leave(binding) => self.finish(binding),
            }
        }
    }

    fn successors(&self, binding: BindingId) -> Vec<BindingId> {
        if let Some(implementing) = self.impl_methods_of_trait.get(&binding) {
            return implementing.clone();
        }
        let Some(references) = self.references.get(&binding) else {
            return Vec::new();
        };
        let named_traits = references
            .written_types()
            .named()
            .iter()
            .filter(|named| self.impl_methods_of_trait.contains_key(named));
        references
            .globals()
            .iter()
            .chain(named_traits)
            .copied()
            .collect()
    }

    fn finish(&mut self, binding: BindingId) {
        if self.impl_method_depths.contains_key(&binding) {
            self.order.push(binding);
        }
    }
}
