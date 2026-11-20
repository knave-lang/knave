use rustc_hash::FxHashMap;

use super::dependency_graph::DependencyGraph;
use crate::core::{BindingId, ItemKind, References};
use crate::expand::Program;

pub(super) struct ConstructedTypes<'d> {
    enum_of_variant: FxHashMap<BindingId, BindingId>,
    dependencies: &'d DependencyGraph,
}

impl<'d> ConstructedTypes<'d> {
    pub(super) fn new(program: &Program, dependencies: &'d DependencyGraph) -> Self {
        Self {
            enum_of_variant: Self::enum_of_variant(program),
            dependencies,
        }
    }

    pub(super) fn by_method(&self, method: BindingId) -> Vec<BindingId> {
        let mut built: Vec<BindingId> = self
            .dependencies
            .reachable_from(&[method])
            .into_iter()
            .filter_map(|reached| self.dependencies.references(reached))
            .flat_map(References::constructions)
            .map(|construction| self.type_built_by(*construction))
            .collect();
        built.sort_unstable();
        built.dedup();
        built
    }

    fn enum_of_variant(program: &Program) -> FxHashMap<BindingId, BindingId> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Enum(binding, _, variants, ..) => Some((*binding, variants)),
                _ => None,
            })
            .flat_map(|(binding, variants)| {
                variants
                    .iter()
                    .map(move |variant| (variant.binding, binding))
            })
            .collect()
    }

    fn type_built_by(&self, construction: BindingId) -> BindingId {
        self.enum_of_variant
            .get(&construction)
            .copied()
            .unwrap_or(construction)
    }
}

#[cfg(test)]
mod tests {
    use super::ConstructedTypes;
    use crate::types::check_order::CheckOrder;
    use crate::types::graph::DefinitionGraph;
    use crate::types::typed_source::TypedSource;

    struct ConstructedTypesFixture {
        source: TypedSource,
    }

    impl ConstructedTypesFixture {
        const SOURCE: &str = "
            (defstruct (labeled t) :v t)
            (defstruct (positional t) :v t)
            (defstruct (passed t) :v t)
            (defstruct (distant t) :v t)
            (defenum (shape t) (dot t))
            (defn near (v) (build-distant v))
            (defn build-distant (v) (->distant :v v))
            (trait probe (probe (fn ((ref self)) bool)))
            (impl probe (vec a)
              (probe (items)
                (do (local _labeled (->labeled :v 1))
                    (local _positional (->positional 1))
                    (local build ->passed)
                    (local _passed (build 1))
                    (local _distant (near 1))
                    (local _shape (shape/dot 1))
                    (= (len items) 0))))
            (defn main () (println (probe (ref (vec 1)))))";

        fn new() -> Self {
            Self {
                source: TypedSource::expand(Self::SOURCE),
            }
        }

        fn built_by_probe(&self) -> Vec<String> {
            let program = self.source.program();
            let graph = DefinitionGraph::of(program);
            let method = CheckOrder::impl_methods(program)
                .find(|method| program.bindings.get(*method).name() == "probe")
                .expect("the source implements probe");
            let mut names: Vec<String> = ConstructedTypes::new(program, graph.dependencies())
                .by_method(method)
                .into_iter()
                .map(|built| program.bindings.get(built).name().to_string())
                .collect();
            names.sort();
            names
        }
    }

    #[test]
    fn a_method_builds_each_type_it_constructs_directly_or_through_definitions() {
        assert_eq!(
            ConstructedTypesFixture::new().built_by_probe(),
            ["distant", "labeled", "passed", "positional", "shape"]
        );
    }
}
