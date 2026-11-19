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
    use crate::types::typed_source::TypedSource;

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

    struct ChainFixture;

    impl ChainFixture {
        const HELD: &str = "{held}";

        const TOP_DOWN_CHAIN: &str = "
            (defn main () (println (t3 (e3/full3 (str \"x\")))))
            (defenum (e3 a) (full3 a) empty3)
            (trait t3 (t3 (fn (self) bool)))
            (impl t3 (e3 {held}) (t3 (w) (match w ((full3 v) (t2 (e2/full2 v))) (empty3 true))))
            (defenum (e2 a) (full2 a) empty2)
            (trait t2 (t2 (fn (self) bool)))
            (impl t2 (e2 {held}) (t2 (w) (match w ((full2 v) (t1 (ref (->w1 v)))) (empty2 true))))
            (defstruct (w1 a) :value a)
            (trait t1 (t1 (fn ((ref self)) bool)))
            (impl t1 (w1 {held}) (t1 (w) (do (local _copy (clone (:value w))) true)))";

        const DISPLAY_MAIN: &str = "
            (defn main () (println (->b2 :inner (->b1 :inner (->b0 :value (str \"x\"))))))";

        const DISPLAY_IMPLS_OUTERMOST_FIRST: [&str; 3] = [
            "(defstruct (b2 a) :inner (b1 a))
            (display (b2 {held}) (x) \"[\" (:inner x) \"]\")",
            "(defstruct (b1 a) :inner (b0 a))
            (display (b1 {held}) (x) \"[\" (:inner x) \"]\")",
            "(defstruct (b0 a) :value a)
            (display (b0 {held}) (x) \"<\" (:value x) \">\")",
        ];

        const SIBLING_METHOD_CHAIN: &str = "
            (defn main () (println (t2 (ref (->w2 :inner (->w1 :value (str \"x\")))))))
            (defstruct (w2 a) :inner (w1 a))
            (trait t2 (t2 (fn ((ref self)) bool)))
            (impl t2 (w2 {held}) (t2 (w) (first (ref (:inner w)))))
            (defstruct (w1 a) :value a)
            (trait pair-check (first (fn ((ref self)) bool)) (second (fn ((ref self)) bool)))
            (impl pair-check (w1 {held})
              (first (w) true)
              (second (w) (do (local _copy (clone (:value w))) true)))";

        fn display_chain(impls: impl Iterator<Item = &'static str>) -> String {
            std::iter::once(Self::DISPLAY_MAIN)
                .chain(impls)
                .collect::<Vec<_>>()
                .join("\n")
        }

        fn checks_in_one_round(chain: &str) -> bool {
            Self::inferred_expressions(chain, "a") == Self::inferred_expressions(chain, "string")
        }

        fn inferred_expressions(chain: &str, held: &str) -> usize {
            let source = chain.replace(Self::HELD, held);
            TypedSource::expand(&source)
                .typed()
                .checking
                .inferred_expressions
        }
    }

    #[test]
    fn a_top_down_chain_of_impls_whose_bodies_need_a_bound_checks_in_one_round() {
        assert!(ChainFixture::checks_in_one_round(
            ChainFixture::TOP_DOWN_CHAIN
        ));
    }

    #[test]
    fn a_top_down_chain_of_display_impls_checks_in_one_round() {
        let impls = ChainFixture::DISPLAY_IMPLS_OUTERMOST_FIRST.into_iter();
        assert!(ChainFixture::checks_in_one_round(
            &ChainFixture::display_chain(impls)
        ));
    }

    #[test]
    fn a_bottom_up_chain_of_display_impls_checks_in_one_round() {
        let impls = ChainFixture::DISPLAY_IMPLS_OUTERMOST_FIRST
            .into_iter()
            .rev();
        assert!(ChainFixture::checks_in_one_round(
            &ChainFixture::display_chain(impls)
        ));
    }

    #[test]
    fn a_call_to_one_method_of_an_impl_waits_for_its_sibling_methods() {
        assert!(ChainFixture::checks_in_one_round(
            ChainFixture::SIBLING_METHOD_CHAIN
        ));
    }
}
