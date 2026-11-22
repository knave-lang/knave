use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, ImplMethod, References};
use crate::expand::Program;

use super::constructed_types::ConstructedTypes;
use super::contained_types::ContainedTypes;
use super::dependency_graph::DependencyGraph;
use super::written_impls::{ImplEntry, WrittenImpls};

pub(super) struct CheckOrder<'p> {
    references: FxHashMap<BindingId, References>,
    impls: WrittenImpls<'p>,
    same_trait_callees: FxHashMap<BindingId, Vec<BindingId>>,
    visited: FxHashSet<BindingId>,
    order: Vec<BindingId>,
}

enum Step {
    Enter(BindingId),
    Leave(BindingId),
}

impl<'p> CheckOrder<'p> {
    pub(super) fn new(
        program: &'p Program,
        references: FxHashMap<BindingId, References>,
        dependencies: &DependencyGraph,
    ) -> Self {
        let impls = WrittenImpls::of(program);
        let contained = ContainedTypes::of(program);
        let constructed = ConstructedTypes::new(program, &references, dependencies);
        let reached = TypesReached::new(&contained, &constructed, &references);
        let same_trait_callees = SameTraitCallees::new(&impls, reached).by_method();
        Self {
            references,
            impls,
            same_trait_callees,
            visited: FxHashSet::default(),
            order: Vec::new(),
        }
    }

    pub(super) fn order(mut self, definitions: impl Iterator<Item = BindingId>) -> Vec<BindingId> {
        let (methods, others): (Vec<BindingId>, Vec<BindingId>) =
            definitions.partition(|binding| self.impls.trait_of(*binding).is_some());
        for method in methods {
            self.visit(method);
        }
        self.order.extend(others);
        self.order
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
        if let Some(implementing) = self.impls.methods_of_trait(binding) {
            return implementing.to_vec();
        }
        let Some(references) = self.references.get(&binding) else {
            return Vec::new();
        };
        let own_trait = self.impls.trait_of(binding);
        let other_traits = references.written_types().named().iter().filter(|named| {
            self.impls.methods_of_trait(**named).is_some() && Some(**named) != own_trait
        });
        let callees = self.same_trait_callees.get(&binding).into_iter().flatten();
        callees
            .chain(references.globals())
            .chain(other_traits)
            .copied()
            .collect()
    }

    fn finish(&mut self, binding: BindingId) {
        if self.impls.trait_of(binding).is_some() {
            self.order.push(binding);
        }
    }
}

struct SameTraitCallees<'i, 'p> {
    impls: &'i WrittenImpls<'p>,
    reached: TypesReached<'i>,
}

impl<'i, 'p> SameTraitCallees<'i, 'p> {
    fn new(impls: &'i WrittenImpls<'p>, reached: TypesReached<'i>) -> Self {
        Self { impls, reached }
    }

    fn by_method(&self) -> FxHashMap<BindingId, Vec<BindingId>> {
        let mut callees = FxHashMap::default();
        for entry in self.impls.entries() {
            for method in entry.methods {
                let reached = self
                    .reached
                    .by(entry, method)
                    .into_iter()
                    .flat_map(|named| self.impls.methods_of_impl(entry.trait_binding, named))
                    .copied()
                    .collect();
                callees.insert(method.binding, reached);
            }
        }
        callees
    }
}

struct TypesReached<'i> {
    contained: &'i ContainedTypes,
    constructed: &'i ConstructedTypes<'i>,
    references: &'i FxHashMap<BindingId, References>,
}

impl<'i> TypesReached<'i> {
    fn new(
        contained: &'i ContainedTypes,
        constructed: &'i ConstructedTypes<'i>,
        references: &'i FxHashMap<BindingId, References>,
    ) -> Self {
        Self {
            contained,
            constructed,
            references,
        }
    }

    fn by(&self, entry: &ImplEntry<'_>, method: &ImplMethod) -> Vec<BindingId> {
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
            .chain(self.constructed.by_method(method.binding))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{SameTraitCallees, TypesReached};
    use crate::core::BindingId;
    use crate::types::constructed_types::ConstructedTypes;
    use crate::types::contained_types::ContainedTypes;
    use crate::types::graph::DefinitionGraph;
    use crate::types::typed_source::TypedSource;
    use crate::types::written_impls::WrittenImpls;

    struct SameTraitFixture {
        source: TypedSource,
    }

    impl SameTraitFixture {
        const SOURCE: &str = "
            (defn main () (println (->built-outer :v (->held-outer :inner (->held :v 1)))))
            (defstruct (built-outer t) :v t)
            (display (built-outer t) (x) \"<\" (->built-inner :v 1) \">\")
            (defstruct (built-inner t) :v t)
            (display (built-inner t) (x) \"<\" (:v x) \">\")
            (defstruct (held t) :v t)
            (display (held t) (x) \"<\" (:v x) \">\")
            (defstruct (held-outer t) :inner (held t))
            (display (held-outer t) (x) \"[\" (:inner x) \"]\")
            (trait probe (probe (fn ((ref self)) bool)))
            (impl probe (held t) (probe (x) true))";

        fn new() -> Self {
            Self {
                source: TypedSource::expand(Self::SOURCE),
            }
        }

        fn callees_of_impl_for(&self, implementor: &str) -> Vec<String> {
            let program = self.source.program();
            let references = DefinitionGraph::walked(program);
            let graph = DefinitionGraph::of(program);
            let impls = WrittenImpls::of(program);
            let contained = ContainedTypes::of(program);
            let constructed = ConstructedTypes::new(program, &references, graph.dependencies());
            let reached = TypesReached::new(&contained, &constructed, &references);
            let callees = SameTraitCallees::new(&impls, reached).by_method();
            let method = self.display_method_of(&impls, implementor);
            let mut names: Vec<String> = callees[&method]
                .iter()
                .map(|callee| self.implementor_of(&impls, *callee))
                .collect();
            names.sort();
            names
        }

        fn display_method_of(&self, impls: &WrittenImpls<'_>, implementor: &str) -> BindingId {
            let program = self.source.program();
            impls
                .entries()
                .iter()
                .filter(|entry| program.bindings.get(entry.trait_binding).name() == "display")
                .find(|entry| {
                    entry
                        .implementor
                        .is_some_and(|head| program.bindings.get(head).name() == implementor)
                })
                .and_then(|entry| entry.methods.first())
                .map(|method| method.binding)
                .expect("the source displays the implementor")
        }

        fn implementor_of(&self, impls: &WrittenImpls<'_>, method: BindingId) -> String {
            let program = self.source.program();
            let entry = impls
                .entries()
                .iter()
                .find(|entry| {
                    entry
                        .methods
                        .iter()
                        .any(|written| written.binding == method)
                })
                .expect("a callee is an impl method");
            let head = entry
                .implementor
                .expect("a callee's impl names its implementor");
            program.bindings.get(head).name().to_string()
        }
    }

    #[test]
    fn a_method_calls_the_same_trait_on_a_type_it_builds() {
        assert_eq!(
            SameTraitFixture::new().callees_of_impl_for("built-outer"),
            ["built-inner"]
        );
    }

    #[test]
    fn a_method_calls_the_same_trait_on_a_type_its_implementor_holds() {
        assert_eq!(
            SameTraitFixture::new().callees_of_impl_for("held-outer"),
            ["held"]
        );
    }

    #[test]
    fn a_method_on_a_type_that_holds_and_builds_nothing_calls_no_impl() {
        assert!(
            SameTraitFixture::new()
                .callees_of_impl_for("built-inner")
                .is_empty()
        );
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
