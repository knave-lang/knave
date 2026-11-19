use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, ImplMethod, ItemKind, References};
use crate::expand::Program;

pub(super) struct CheckOrder<'a> {
    references: &'a FxHashMap<BindingId, References>,
    impl_methods_of_trait_method: FxHashMap<BindingId, Vec<BindingId>>,
    impl_methods: FxHashSet<BindingId>,
    visited: FxHashSet<BindingId>,
    order: Vec<BindingId>,
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
        let mut walk = Self {
            references,
            impl_methods_of_trait_method: Self::impl_methods_by_trait_method(program),
            impl_methods: Self::impl_methods(program).collect(),
            visited: FxHashSet::default(),
            order: Vec::with_capacity(references.len()),
        };
        let (methods, others): (Vec<BindingId>, Vec<BindingId>) =
            definitions.partition(|binding| walk.impl_methods.contains(binding));
        for method in methods {
            walk.visit(method);
        }
        walk.order.extend(others);
        walk.order
    }

    pub(super) fn impl_methods<'p>(
        program: &'p Program,
    ) -> impl Iterator<Item = BindingId> + use<'p> {
        Self::impls(program).flatten().map(|method| method.binding)
    }

    fn impls<'p>(program: &'p Program) -> impl Iterator<Item = &'p [ImplMethod]> + use<'p> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Impl(_, _, _, methods) => Some(methods.as_slice()),
                _ => None,
            })
    }

    fn impl_methods_by_trait_method(program: &Program) -> FxHashMap<BindingId, Vec<BindingId>> {
        let mut by_trait_method: FxHashMap<BindingId, Vec<BindingId>> = FxHashMap::default();
        for methods in Self::impls(program) {
            for method in methods {
                by_trait_method
                    .entry(method.trait_method)
                    .or_default()
                    .extend(methods.iter().map(|sibling| sibling.binding));
            }
        }
        by_trait_method
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
        let referenced = self
            .references
            .get(&binding)
            .map(References::globals)
            .unwrap_or_default();
        let implementing = self
            .impl_methods_of_trait_method
            .get(&binding)
            .map(Vec::as_slice)
            .unwrap_or_default();
        referenced.iter().chain(implementing).copied().collect()
    }

    fn finish(&mut self, binding: BindingId) {
        if self.impl_methods.contains(&binding) {
            self.order.push(binding);
        }
    }
}
