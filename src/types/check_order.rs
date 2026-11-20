use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, ImplMethod, ItemKind, References, TypeExpr, TypeExprKind};
use crate::expand::Program;

use super::constructed_types::ConstructedTypes;
use super::contained_types::ContainedTypes;

pub(super) struct CheckOrder<'a> {
    references: &'a FxHashMap<BindingId, References>,
    impl_methods_of_trait: FxHashMap<BindingId, Vec<BindingId>>,
    trait_of_impl_method: FxHashMap<BindingId, BindingId>,
    same_trait_callees: FxHashMap<BindingId, Vec<BindingId>>,
    visited: FxHashSet<BindingId>,
    order: Vec<BindingId>,
}

struct ImplEntry<'p> {
    trait_binding: BindingId,
    implementor: Option<BindingId>,
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
        let impls = Self::impl_entries(program);
        let mut walk = Self {
            references,
            impl_methods_of_trait: Self::impl_methods_by_trait(&impls),
            trait_of_impl_method: Self::trait_of_impl_method(&impls),
            same_trait_callees: SameTraitCallees::new(
                &impls,
                &ContainedTypes::of(program),
                references,
            )
            .of(&ConstructedTypes::of(program)),
            visited: FxHashSet::default(),
            order: Vec::with_capacity(references.len()),
        };
        let (methods, others): (Vec<BindingId>, Vec<BindingId>) =
            definitions.partition(|binding| walk.trait_of_impl_method.contains_key(binding));
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

    fn impl_entries<'p>(program: &'p Program) -> Vec<ImplEntry<'p>> {
        Self::impls(program)
            .map(|(trait_binding, implementor, methods)| ImplEntry {
                trait_binding,
                implementor: Self::head(implementor),
                methods,
            })
            .collect()
    }

    fn head(implementor: &TypeExpr) -> Option<BindingId> {
        match implementor.kind() {
            TypeExprKind::Named(binding, _) => Some(*binding),
            _ => None,
        }
    }

    fn trait_of_impl_method(impls: &[ImplEntry<'_>]) -> FxHashMap<BindingId, BindingId> {
        impls
            .iter()
            .flat_map(|entry| {
                entry
                    .methods
                    .iter()
                    .map(|method| (method.binding, entry.trait_binding))
            })
            .collect()
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
        let own_trait = self.trait_of_impl_method.get(&binding);
        let other_traits = references.written_types().named().iter().filter(|named| {
            self.impl_methods_of_trait.contains_key(named) && Some(*named) != own_trait
        });
        let callees = self.same_trait_callees.get(&binding).into_iter().flatten();
        callees
            .chain(references.globals())
            .chain(other_traits)
            .copied()
            .collect()
    }

    fn finish(&mut self, binding: BindingId) {
        if self.trait_of_impl_method.contains_key(&binding) {
            self.order.push(binding);
        }
    }
}

struct SameTraitCallees<'i, 'p> {
    impls: &'i [ImplEntry<'p>],
    contained: &'i ContainedTypes,
    references: &'i FxHashMap<BindingId, References>,
}

impl<'i, 'p> SameTraitCallees<'i, 'p> {
    fn new(
        impls: &'i [ImplEntry<'p>],
        contained: &'i ContainedTypes,
        references: &'i FxHashMap<BindingId, References>,
    ) -> Self {
        Self {
            impls,
            contained,
            references,
        }
    }

    fn of(&self, constructed: &ConstructedTypes) -> FxHashMap<BindingId, Vec<BindingId>> {
        let methods_by_impl = self.methods_by_trait_and_implementor();
        let mut callees = FxHashMap::default();
        for entry in self.impls {
            for method in entry.methods {
                let reached = self
                    .types_reached(entry, method, constructed)
                    .into_iter()
                    .filter_map(|named| methods_by_impl.get(&(entry.trait_binding, named)))
                    .flatten()
                    .copied()
                    .collect();
                callees.insert(method.binding, reached);
            }
        }
        callees
    }

    fn methods_by_trait_and_implementor(
        &self,
    ) -> FxHashMap<(BindingId, BindingId), Vec<BindingId>> {
        let mut by_impl: FxHashMap<(BindingId, BindingId), Vec<BindingId>> = FxHashMap::default();
        for entry in self.impls {
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

    fn types_reached(
        &self,
        entry: &ImplEntry<'_>,
        method: &ImplMethod,
        constructed: &ConstructedTypes,
    ) -> Vec<BindingId> {
        let contained = entry
            .implementor
            .map(|implementor| self.contained.in_type(implementor))
            .unwrap_or_default();
        let written = self
            .references
            .get(&method.binding)
            .map(|references| references.written_types().named())
            .unwrap_or_default();
        contained
            .iter()
            .chain(written)
            .copied()
            .chain(constructed.in_lambda(&method.lambda))
            .collect()
    }
}
