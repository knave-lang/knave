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
            (defn main () (println (->b2 :inner (b1/wrap (->b0 :value (str \"x\"))))))";

        const DISPLAY_IMPLS_OUTERMOST_FIRST: [&str; 3] = [
            "(defstruct (b2 a) :inner (b1 a))
            (display (b2 {held}) (x) \"[\" (:inner x) \"]\")",
            "(defenum (b1 a) (wrap (b0 a)))
            (display (b1 {held}) (x) (match x ((b1/wrap inner) (str inner))))",
            "(defstruct (b0 a) :value a)
            (display (b0 {held}) (x) \"<\" (:value x) \">\")",
        ];

        const BUILT_CHAIN: &str = "
            (defn main () (println (->a1 :v 1)))
            (defstruct (a1 t) :v t)
            (display (a1 {held}) (x) \"<\" (->a2 :v (:v x)) \">\")
            (defstruct (a2 t) :v t)
            (display (a2 {held}) (x) \"<\" (->a3 :v (:v x)) \">\")
            (defstruct (a3 t) :v t)
            (display (a3 {held}) (x) \"<\" (:v x) \">\")";

        const INVERTED_DEPTHS: &str = "
            (defn main () (println (->aa :v 1)))
            (defstruct (dd t) :v t)
            (display (dd {held}) (x) \"d\" (:v x))
            (defstruct (bb t) :inner (dd t))
            (display (bb {held}) (x) \"b\" (:inner x))
            (defstruct (aa t) :v t)
            (display (aa {held}) (x) \"a\" (->bb :inner (->dd :v (:v x))))";

        const VARIANT_CHAIN: &str = "
            (defn main () (println (->c1 :v 1)))
            (defstruct (c1 t) :v t)
            (display (c1 {held}) (x) \"<\" (c2/wrap (:v x)) \">\")
            (defenum (c2 t) (wrap t))
            (display (c2 {held}) (x) (match x ((c2/wrap v) (str v))))";

        const WRITTEN_CHAIN: &str = "
            (defn main () (do (d2-of true) (println (->d1 :v (len \"x\")))))
            (defn d2-of (v) (->d2 :v v))
            (defstruct (d1 t) :v t)
            (display (d1 {held}) (x) (do (local (inner (d2 {held})) (d2-of (:v x))) inner))
            (defstruct (d2 t) :v t)
            (display (d2 {held}) (x) \"<\" (:v x) \">\")";

        const POSITIONAL_CHAIN: &str = "
            (defn main () (println (->a1 :v 1)))
            (defstruct (a1 t) :v t)
            (display (a1 {held}) (x) \"<\" (->a2 (:v x)) \">\")
            (defstruct (a2 t) :v t)
            (display (a2 {held}) (x) \"<\" (->a3 (:v x)) \">\")
            (defstruct (a3 t) :v t)
            (display (a3 {held}) (x) \"<\" (:v x) \">\")";

        const CONSTRUCTOR_VALUE_CHAIN: &str = "
            (defn main () (println (->a1 :v 1)))
            (defstruct (a1 t) :v t)
            (display (a1 {held}) (x) \"<\" (do (local build ->a2) (build (:v x))) \">\")
            (defstruct (a2 t) :v t)
            (display (a2 {held}) (x) \"<\" (:v x) \">\")";

        const FUNCTION_CHAIN: &str = "
            (defn main () (do (wrap-a2 true) (println (->a1 :v 1))))
            (defn wrap-a2 (v) (build-a2 v))
            (defn build-a2 (v) (->a2 :v v))
            (defstruct (a1 t) :v t)
            (display (a1 {held}) (x) \"<\" (wrap-a2 (:v x)) \">\")
            (defstruct (a2 t) :v t)
            (display (a2 {held}) (x) \"<\" (:v x) \">\")";

        const RECURSIVE_CALLS: &str = "
            (defn main () (println (->a1 :v 1)))
            (defn countdown (n) (if (= n 0) 0 (countdown (- n 1))))
            (defn spin (n) (if (= n 0) (->a2 :v 0) (spin (- n 1))))
            (defn ping (n) (if (= n 0) 0 (pong (- n 1))))
            (defn pong (n) (if (= n 0) 0 (ping (- n 1))))
            (defstruct (a1 t) :v t)
            (display (a1 {held}) (x) \"<\" (:v x) (countdown 2) (spin 2) (ping 3) \">\")
            (defstruct (a2 t) :v t)
            (display (a2 {held}) (x) \"<\" (:v x) \">\")";

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
            Self::checks_in_one_round_holding(chain, "string")
        }

        fn checks_in_one_round_holding(chain: &str, concrete: &str) -> bool {
            Self::inferred_expressions(chain, "a") == Self::inferred_expressions(chain, concrete)
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

    #[test]
    fn a_chain_of_display_impls_that_build_the_next_type_checks_in_one_round() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::BUILT_CHAIN,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_builds_a_deeper_type_checks_after_its_impl() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::INVERTED_DEPTHS,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_builds_an_enum_variant_checks_after_the_enum_impl() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::VARIANT_CHAIN,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_writes_a_type_checks_after_that_type_impl() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::WRITTEN_CHAIN,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_calls_a_constructor_without_labels_checks_after_its_impl() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::POSITIONAL_CHAIN,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_names_a_constructor_as_a_value_checks_after_its_impl() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::CONSTRUCTOR_VALUE_CHAIN,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_builds_a_type_through_functions_checks_after_its_impl() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::FUNCTION_CHAIN,
            "i64"
        ));
    }

    #[test]
    fn an_impl_that_calls_recursive_functions_checks_after_the_impls_they_build() {
        assert!(ChainFixture::checks_in_one_round_holding(
            ChainFixture::RECURSIVE_CALLS,
            "i64"
        ));
    }
}
