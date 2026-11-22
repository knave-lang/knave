use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::check_order::CheckOrder;
use super::deferred_definitions::DeferredDefinitions;
use super::dependency_graph::DependencyGraph;
use super::reached_traits::ReachedTraits;
use super::trait_reach::TraitReach;
use super::written_bodies::WrittenBodies;
use crate::core::{BindingArena, BindingId, Expr, ItemKind, References};
use crate::expand::Program;
use crate::source::Span;

pub struct Definition<'p> {
    pub binding: BindingId,
    pub value: &'p Expr,
    pub span: Span,
}

pub struct DefinitionGraph<'p> {
    definitions: Vec<Definition<'p>>,
    dependencies: DependencyGraph,
    components: Vec<Vec<BindingId>>,
    reach: ProgramReach,
}

pub struct ProgramReach {
    unreached: Rc<FxHashSet<BindingId>>,
    traits: ReachedTraits,
    written: WrittenBodies,
}

impl ProgramReach {
    pub fn unreached(&self) -> Rc<FxHashSet<BindingId>> {
        Rc::clone(&self.unreached)
    }

    pub fn traits(&self) -> &ReachedTraits {
        &self.traits
    }

    pub fn written_bodies(&self) -> &WrittenBodies {
        &self.written
    }
}

impl<'p> DefinitionGraph<'p> {
    pub fn of(program: &'p Program) -> Self {
        Self::deferring(
            program,
            DeferredDefinitions::of(program, &WrittenBodies::of(&program.modules)),
            program.prelude_macro_helpers(),
        )
    }

    fn deferring(
        program: &'p Program,
        mut deferred: FxHashSet<BindingId>,
        macro_helpers: FxHashSet<BindingId>,
    ) -> Self {
        let walked =
            ReferenceWalk::new(&program.bindings, &deferred).run(Self::definitions_in(program));
        deferred.retain(|binding| !walked.contains_key(binding));
        let reached_traits = TraitReach::of(&program.modules, walked.values());
        let mut definitions = Vec::with_capacity(Self::item_count(program));
        definitions.extend(
            Self::definitions_in(program)
                .filter(|definition| walked.contains_key(&definition.binding)),
        );
        let written = WrittenBodies::of(&program.modules);
        let dependencies = DependencyGraph::build(&walked, macro_helpers, &written);
        let order = CheckOrder::new(program, walked, &dependencies)
            .order(definitions.iter().map(|definition| definition.binding));
        let components = dependencies.components(&order);
        Self {
            definitions,
            dependencies,
            components,
            reach: ProgramReach {
                unreached: Rc::new(deferred),
                traits: reached_traits,
                written,
            },
        }
    }

    fn item_count(program: &Program) -> usize {
        program
            .modules
            .iter()
            .map(|module| module.items.len())
            .sum()
    }

    fn definitions_in(program: &'p Program) -> impl Iterator<Item = Definition<'p>> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Def(binding, value) => Some(Definition {
                    binding: *binding,
                    value,
                    span: item.span,
                }),
                _ => None,
            })
    }

    pub fn definitions(&self) -> &[Definition<'p>] {
        &self.definitions
    }

    pub fn reach(&self) -> &ProgramReach {
        &self.reach
    }

    pub fn dependencies(&self) -> &DependencyGraph {
        &self.dependencies
    }

    pub fn order(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.definitions.iter().map(|definition| definition.binding)
    }

    pub fn components(&self) -> &[Vec<BindingId>] {
        &self.components
    }
}

struct ReferenceWalk<'a, 'p> {
    bindings: &'a BindingArena,
    deferred: &'a FxHashSet<BindingId>,
    waiting: FxHashMap<BindingId, &'p Expr>,
    reached: Vec<BindingId>,
    walked: FxHashMap<BindingId, References>,
}

impl<'a, 'p> ReferenceWalk<'a, 'p> {
    fn new(bindings: &'a BindingArena, deferred: &'a FxHashSet<BindingId>) -> Self {
        Self {
            bindings,
            deferred,
            waiting: FxHashMap::default(),
            reached: Vec::new(),
            walked: FxHashMap::default(),
        }
    }

    fn run(
        mut self,
        definitions: impl Iterator<Item = Definition<'p>>,
    ) -> FxHashMap<BindingId, References> {
        for root in self.set_aside_deferred(definitions) {
            self.walk(root.binding, root.value);
        }
        while let Some(binding) = self.reached.pop() {
            if let Some(value) = self.waiting.remove(&binding) {
                self.walk(binding, value);
            }
        }
        self.walked
    }

    fn set_aside_deferred(
        &mut self,
        definitions: impl Iterator<Item = Definition<'p>>,
    ) -> Vec<Definition<'p>> {
        let (deferred, roots): (Vec<_>, Vec<_>) =
            definitions.partition(|definition| self.deferred.contains(&definition.binding));
        self.waiting = deferred
            .into_iter()
            .map(|definition| (definition.binding, definition.value))
            .collect();
        roots
    }

    fn walk(&mut self, binding: BindingId, value: &Expr) {
        let references = References::of(value, self.bindings);
        self.reached.extend(
            references
                .globals()
                .iter()
                .filter(|global| self.waiting.contains_key(global)),
        );
        self.walked.insert(binding, references);
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use rustc_hash::FxHashSet;

    use super::DefinitionGraph;
    use crate::core::BindingId;
    use crate::expand::Program;
    use crate::modules::ModuleSources;
    use crate::types::written_impls::WrittenImpls;
    use crate::types::{TypedProgram, TypedSource};

    #[test]
    fn reserves_the_definition_list_once_for_every_item() {
        let source = TypedSource::expand("(def rate 3)\n(def total (* rate 2))\n");
        let program = source.program();
        let items = program.modules.iter().flat_map(|module| &module.items);
        let graph = DefinitionGraph::of(program);
        assert_eq!(graph.definitions.capacity(), items.count());
    }

    #[test]
    fn an_empty_program_types_no_prelude_macro_helper() {
        let source = TypedSource::expand(PreludeFixture::EMPTY_PROGRAM);
        let helpers = source.program().prelude_macro_helpers();
        assert!(!helpers.is_empty());
        assert!(PreludeFixture::typed_bindings(&source.typed()).is_disjoint(&helpers));
    }

    #[test]
    fn every_prelude_definition_outside_compile_time_code_type_checks_once() {
        let source = TypedSource::expand(PreludeFixture::EMPTY_PROGRAM);
        let graph = DefinitionGraph::deferring(
            source.program(),
            FxHashSet::default(),
            FxHashSet::default(),
        );
        let typed = PreludeFixture::typed_bindings(&source.check(&graph));
        let unchecked = PreludeFixture::prelude_definitions(source.program())
            .into_iter()
            .filter(|binding| !graph.dependencies().is_compile_time_only(*binding))
            .find(|binding| !typed.contains(binding));
        assert_eq!(unchecked, None);
    }

    #[test]
    fn an_empty_program_types_no_prelude_function_it_does_not_reach() {
        assert!(!PreludeFixture::types(PreludeFixture::EMPTY_PROGRAM, "sum"));
    }

    #[test]
    fn a_reached_prelude_function_is_typed_with_what_it_reaches() {
        assert!(PreludeFixture::types(PreludeFixture::SUMMING, "sum"));
        assert!(PreludeFixture::types(PreludeFixture::SUMMING, "add"));
    }

    #[test]
    fn an_arm_that_reads_an_outer_name_reaches_that_prelude_function() {
        const ARM_NAMES_SUM: &str = "(enum total sum)
            (defn main ()
              (match total/sum
                (sum (sum (vec 1 2)))))";
        assert!(PreludeFixture::types(ARM_NAMES_SUM, "sum"));
    }

    #[test]
    fn an_arm_that_leaves_an_outer_name_unread_reaches_nothing() {
        const ARM_LEAVES_SUM_UNREAD: &str = "(enum total sum)
            (defn main ()
              (match total/sum
                (sum (println 1))))";
        assert!(!PreludeFixture::types(ARM_LEAVES_SUM_UNREAD, "sum"));
    }

    #[test]
    fn a_labeled_arm_that_reads_an_outer_name_reaches_that_prelude_function() {
        const LABELED_ARM_NAMES_SUM: &str = "(enum total sum)
            (defstruct holder :inner total)
            (defn main ()
              (match (->holder :inner total/sum)
                ((holder :inner sum) (sum (vec 1 2)))))";
        assert!(PreludeFixture::types(LABELED_ARM_NAMES_SUM, "sum"));
    }

    #[test]
    fn a_nested_arm_that_reads_an_outer_name_reaches_that_prelude_function() {
        const NESTED_ARM_NAMES_SUM: &str = "(enum total sum)
            (enum wrapped (holds total))
            (defn main ()
              (match (wrapped/holds total/sum)
                ((holds sum) (sum (vec 1 2)))))";
        assert!(PreludeFixture::types(NESTED_ARM_NAMES_SUM, "sum"));
    }

    #[test]
    fn a_checked_root_is_typed_though_nothing_reaches_it() {
        let mut source = TypedSource::expand(PreludeFixture::EMPTY_PROGRAM);
        let sum = PreludeFixture::prelude_definition(source.program(), "sum");
        source.check_also(sum);
        assert!(PreludeFixture::typed_bindings(&source.typed()).contains(&sum));
    }

    #[test]
    fn the_string_view_is_typed_though_nothing_reaches_it() {
        let source = TypedSource::expand(PreludeFixture::EMPTY_PROGRAM);
        let view = source
            .program()
            .string_view
            .expect("the prelude views strings");
        assert!(PreludeFixture::typed_bindings(&source.typed()).contains(&view));
    }

    #[test]
    #[should_panic(expected = "the definition graph missed a reference")]
    fn a_reference_the_walk_missed_stops_the_checker() {
        let source = TypedSource::expand(PreludeFixture::SUMMING);
        let sum = PreludeFixture::prelude_definition(source.program(), "sum");
        let mut graph = DefinitionGraph::of(source.program());
        graph
            .components
            .retain(|component| !component.contains(&sum));
        graph.reach.unreached = Rc::new(FxHashSet::from_iter([sum]));
        source.check(&graph);
    }

    #[test]
    fn the_graph_names_a_definition_left_undecoded_as_unreached() {
        let source = TypedSource::expand_on_a_loaded_prelude(PreludeFixture::EMPTY_PROGRAM);
        let program = source.program();
        let undecoded = program.pending_definitions.undecoded().next();
        let undecoded = undecoded.expect("an empty program leaves definitions undecoded");
        assert!(
            DefinitionGraph::of(program)
                .reach()
                .unreached()
                .contains(&undecoded)
        );
    }

    #[test]
    fn the_graph_names_what_it_left_unreached() {
        let source = TypedSource::expand(PreludeFixture::EMPTY_PROGRAM);
        let sum = PreludeFixture::prelude_definition(source.program(), "sum");
        assert!(
            DefinitionGraph::of(source.program())
                .reach()
                .unreached()
                .contains(&sum)
        );
    }

    #[test]
    fn the_graph_leaves_out_of_unreached_what_a_program_reaches() {
        let source = TypedSource::expand(PreludeFixture::SUMMING);
        let sum = PreludeFixture::prelude_definition(source.program(), "sum");
        assert!(
            !DefinitionGraph::of(source.program())
                .reach()
                .unreached()
                .contains(&sum)
        );
    }

    #[test]
    fn an_impl_method_is_checked_before_a_function_written_ahead_of_its_impl() {
        let fixture = ImplOrderFixture::new();
        assert!(fixture.impl_method_position() < fixture.position_of("early"));
    }

    struct ImplOrderFixture {
        source: TypedSource,
    }

    impl ImplOrderFixture {
        const SOURCE: &str = "
            (trait probe (probe (fn ((ref self)) bool)))
            (defn early (items) (probe items))
            (impl probe (vec a) (probe (items) (= (len items) 0)))
            (defn main () (println (early (ref (vec 1)))))";

        fn new() -> Self {
            Self {
                source: TypedSource::expand(Self::SOURCE),
            }
        }

        fn impl_method_position(&self) -> usize {
            let program = self.source.program();
            let method = WrittenImpls::of(program)
                .entries()
                .iter()
                .flat_map(|entry| entry.methods)
                .map(|method| method.binding)
                .find(|method| program.bindings.get(*method).name() == "probe")
                .expect("the source implements probe");
            self.component_holding(method)
        }

        fn position_of(&self, name: &str) -> usize {
            let program = self.source.program();
            let binding = program
                .modules
                .iter()
                .flat_map(|module| module.definitions())
                .map(|(binding, _)| binding)
                .find(|binding| program.bindings.get(*binding).name() == name)
                .expect("the source defines the name");
            self.component_holding(binding)
        }

        fn component_holding(&self, binding: BindingId) -> usize {
            DefinitionGraph::of(self.source.program())
                .components()
                .iter()
                .position(|component| component.contains(&binding))
                .expect("the graph checks the binding")
        }
    }

    struct PreludeFixture;

    impl PreludeFixture {
        const EMPTY_PROGRAM: &str = "(defn main ())";
        const SUMMING: &str = "(defn main () (println (sum (vec 1 2))))";

        fn types(program: &str, name: &str) -> bool {
            let source = TypedSource::expand(program);
            let binding = Self::prelude_definition(source.program(), name);
            Self::typed_bindings(&source.typed()).contains(&binding)
        }

        fn prelude_definition(program: &Program, name: &str) -> BindingId {
            Self::prelude_definitions(program)
                .into_iter()
                .find(|binding| program.bindings.get(*binding).name() == name)
                .expect("the prelude defines the name")
        }

        fn prelude_definitions(program: &Program) -> Vec<BindingId> {
            program
                .modules
                .iter()
                .filter(|module| ModuleSources::is_prelude(&module.path))
                .flat_map(|module| module.definitions())
                .map(|(binding, _)| binding)
                .collect()
        }

        fn typed_bindings(program: &TypedProgram) -> FxHashSet<BindingId> {
            program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .map(|definition| definition.binding)
                .collect()
        }
    }
}
