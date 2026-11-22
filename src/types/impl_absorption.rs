use rustc_hash::FxHashMap;

use super::impl_bound_seeds::ImplBoundSeeds;
use super::impl_position::ImplPosition;
use super::structural_uses::ImplBoundUse;
use super::ty::{Bound, GenericParam};
use super::typed_trait_impl::TypedTraitImpl;
use super::unifier::Unifier;
use crate::core::BindingId;

pub(super) struct ImplAbsorption {
    impl_of_method: FxHashMap<BindingId, ImplPosition>,
    unchecked_methods: Vec<usize>,
    grown: Vec<ImplPosition>,
    seeds: ImplBoundSeeds,
    reruns: bool,
}

impl ImplAbsorption {
    pub(super) fn new(impls: &[TypedTraitImpl]) -> Self {
        let impl_of_method = ImplPosition::all(impls.len())
            .flat_map(|position| {
                let methods = &impls[position.index()].methods;
                methods.iter().map(move |method| (method.binding, position))
            })
            .collect();
        Self {
            impl_of_method,
            unchecked_methods: impls
                .iter()
                .map(|implementation| implementation.methods.len())
                .collect(),
            grown: Vec::new(),
            seeds: ImplBoundSeeds::default(),
            reruns: false,
        }
    }

    pub(super) fn finish_methods(&mut self, members: &[BindingId]) -> Vec<ImplPosition> {
        members
            .iter()
            .filter_map(|member| self.finish_method(*member))
            .collect()
    }

    pub(super) fn every_impl(&self) -> Vec<ImplPosition> {
        ImplPosition::all(self.unchecked_methods.len()).collect()
    }

    fn finish_method(&mut self, method: BindingId) -> Option<ImplPosition> {
        let position = *self.impl_of_method.get(&method)?;
        let unchecked = &mut self.unchecked_methods[position.index()];
        *unchecked -= 1;
        (*unchecked == 0).then_some(position)
    }

    pub(super) fn record(
        &mut self,
        position: ImplPosition,
        params: &[GenericParam],
        absorbed: &[Vec<Bound>],
    ) {
        if Self::adds_bounds(params, absorbed) {
            self.grown.push(position);
        }
        self.seeds.record(position, params, absorbed);
    }

    pub(super) fn settle(
        &mut self,
        uses: &[ImplBoundUse],
        unifier: &Unifier,
        impls: &[TypedTraitImpl],
    ) {
        let grown = std::mem::take(&mut self.grown);
        self.reruns |= grown
            .iter()
            .any(|position| Self::was_selected(&impls[position.index()], uses, unifier));
    }

    pub(super) fn into_rerun(self) -> Option<ImplBoundSeeds> {
        self.reruns.then_some(self.seeds)
    }

    fn adds_bounds(params: &[GenericParam], absorbed: &[Vec<Bound>]) -> bool {
        params
            .iter()
            .zip(absorbed)
            .any(|(param, bounds)| bounds.iter().any(|bound| !param.bounds.contains(bound)))
    }

    fn was_selected(
        implementation: &TypedTraitImpl,
        uses: &[ImplBoundUse],
        unifier: &Unifier,
    ) -> bool {
        uses.iter().any(|pending| {
            pending.bound.binding == implementation.trait_binding
                && (implementation.implementor)
                    .shares_head_with(&unifier.resolve(&pending.implementor))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ImplAbsorption;
    use crate::core::BindingId;
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "
        (trait pair-check
          (first-check (fn ((ref self)) bool))
          (second-check (fn ((ref self)) bool)))
        (impl pair-check (vec a)
          (first-check (items) (= (len items) 0))
          (second-check (items) (= (len items) 1)))
        (defn main () (do))";

    struct AbsorptionFixture {
        declarations: DeclarationsFixture,
    }

    impl AbsorptionFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn absorption(&self) -> ImplAbsorption {
            ImplAbsorption::new(self.declarations.traits().impls())
        }

        fn methods(&self) -> Vec<BindingId> {
            let pair_check = self.declarations.trait_named("pair-check");
            let implementation = self
                .declarations
                .traits()
                .impls()
                .iter()
                .find(|implementation| implementation.trait_binding == pair_check)
                .expect("the source implements pair-check");
            implementation
                .methods
                .iter()
                .map(|method| method.binding)
                .collect()
        }
    }

    #[test]
    fn an_impl_is_finished_by_its_last_method() {
        let fixture = AbsorptionFixture::new();
        let mut absorption = fixture.absorption();
        let finished: Vec<bool> = fixture
            .methods()
            .into_iter()
            .map(|method| !absorption.finish_methods(&[method]).is_empty())
            .collect();
        assert_eq!(finished, vec![false, true]);
    }
}
