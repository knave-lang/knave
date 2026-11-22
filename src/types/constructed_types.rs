use rustc_hash::FxHashMap;

use super::dependency_graph::DependencyGraph;
use crate::core::{BindingId, ItemKind, References};
use crate::expand::Program;

pub(super) struct ConstructedTypes<'d> {
    enum_of_variant: FxHashMap<BindingId, BindingId>,
    references: &'d FxHashMap<BindingId, References>,
    dependencies: &'d DependencyGraph,
}

impl<'d> ConstructedTypes<'d> {
    pub(super) fn new(
        program: &Program,
        references: &'d FxHashMap<BindingId, References>,
        dependencies: &'d DependencyGraph,
    ) -> Self {
        Self {
            enum_of_variant: Self::enum_of_variant(program),
            references,
            dependencies,
        }
    }

    pub(super) fn by_method(&self, method: BindingId) -> Vec<BindingId> {
        let mut built: Vec<BindingId> = self
            .dependencies
            .reachable_from(&[method])
            .into_iter()
            .filter_map(|reached| self.references.get(&reached))
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
    use rustc_hash::{FxHashMap, FxHashSet};

    use super::ConstructedTypes;
    use crate::core::{BindingId, References};
    use crate::types::dependency_graph::DependencyGraph;
    use crate::types::typed_source::TypedSource;
    use crate::types::written_bodies::WrittenBodies;
    use crate::types::written_impls::WrittenImpls;

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
            let references = self.references();
            let dependencies = DependencyGraph::build(
                &references,
                FxHashSet::default(),
                &WrittenBodies::of(&program.modules),
            );
            let method = WrittenImpls::of(program)
                .entries()
                .iter()
                .flat_map(|entry| entry.methods)
                .map(|method| method.binding)
                .find(|method| program.bindings.get(*method).name() == "probe")
                .expect("the source implements probe");
            let mut names: Vec<String> = ConstructedTypes::new(program, &references, &dependencies)
                .by_method(method)
                .into_iter()
                .map(|built| program.bindings.get(built).name().to_string())
                .collect();
            names.sort();
            names
        }

        fn references(&self) -> FxHashMap<BindingId, References> {
            let program = self.source.program();
            program
                .modules
                .iter()
                .flat_map(|module| module.definitions())
                .map(|(binding, value)| (binding, References::of(value, &program.bindings)))
                .collect()
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
