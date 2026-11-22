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
}
