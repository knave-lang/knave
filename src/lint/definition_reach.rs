use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, CoreModule, Item, ItemKind, ModuleId, References};
use crate::expand::Program;

pub struct DefinitionReach {
    unreached: FxHashMap<ModuleId, Vec<UnreachedDefinition>>,
}

pub struct UnreachedDefinition {
    pub binding: BindingId,
    pub readers: Vec<BindingId>,
}

struct ReferenceGraph {
    reads: FxHashMap<BindingId, Vec<BindingId>>,
    readers: FxHashMap<BindingId, Vec<BindingId>>,
    homes: FxHashMap<BindingId, ModuleId>,
}

impl DefinitionReach {
    pub fn of(program: &Program, candidates: FxHashMap<BindingId, ModuleId>) -> Self {
        let graph = ReferenceGraph::of(program);
        let roots =
            graph.reads.keys().copied().filter(|binding| {
                !candidates.contains_key(binding) || graph.read_elsewhere(*binding)
            });
        let reached = graph.reached_from(roots);
        let mut unreached: FxHashMap<ModuleId, Vec<UnreachedDefinition>> = FxHashMap::default();
        for (binding, module) in &candidates {
            if !reached.contains(binding) {
                let readers = graph.readers_of(*binding);
                let definition = UnreachedDefinition {
                    binding: *binding,
                    readers,
                };
                unreached.entry(*module).or_default().push(definition);
            }
        }
        Self { unreached }
    }

    pub fn unreached_in(&self, module: ModuleId) -> &[UnreachedDefinition] {
        self.unreached.get(&module).map_or(&[], Vec::as_slice)
    }

    pub fn candidates(program: &Program) -> FxHashMap<BindingId, ModuleId> {
        program
            .project_modules()
            .flat_map(|module| {
                module
                    .items
                    .iter()
                    .filter_map(move |item| Self::candidate(program, module, item))
            })
            .collect()
    }

    fn candidate(
        program: &Program,
        module: &CoreModule,
        item: &Item,
    ) -> Option<(BindingId, ModuleId)> {
        let ItemKind::Def(binding, _) = item.kind else {
            return None;
        };
        let entry_function = program
            .entry
            .function
            .filter(|_| module.id == program.entry.module);
        let private = !item.hidden && !module.exports().contains(binding);
        let written = !program.bindings.origin(binding).is_introduced()
            && Some(binding) != entry_function
            && !program.compile_time_roots.contains(binding);
        (private && written).then_some((binding, module.id))
    }
}

impl ReferenceGraph {
    fn of(program: &Program) -> Self {
        let mut graph = Self {
            reads: FxHashMap::default(),
            readers: FxHashMap::default(),
            homes: FxHashMap::default(),
        };
        for module in program.project_modules() {
            for (binding, value) in module.definitions() {
                let reads = References::of(value, &program.bindings).into_globals();
                for read in &reads {
                    graph.readers.entry(*read).or_default().push(binding);
                }
                graph.reads.insert(binding, reads);
                graph.homes.insert(binding, module.id);
            }
        }
        graph
    }

    fn read_elsewhere(&self, binding: BindingId) -> bool {
        let home = self.homes.get(&binding);
        self.readers
            .get(&binding)
            .into_iter()
            .flatten()
            .any(|reader| self.homes.get(reader) != home)
    }

    fn reached_from(&self, roots: impl Iterator<Item = BindingId>) -> FxHashSet<BindingId> {
        let mut reached: FxHashSet<BindingId> = FxHashSet::default();
        let mut waiting: Vec<BindingId> = roots.collect();
        while let Some(binding) = waiting.pop() {
            if reached.insert(binding) {
                waiting.extend(self.reads.get(&binding).into_iter().flatten());
            }
        }
        reached
    }

    fn readers_of(&self, binding: BindingId) -> Vec<BindingId> {
        let mut seen: FxHashSet<BindingId> = FxHashSet::from_iter([binding]);
        let direct = self.readers.get(&binding).into_iter().flatten();
        direct
            .copied()
            .filter(|reader| seen.insert(*reader))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::DefinitionReach;
    use crate::core::BindingId;
    use crate::expand::Program;
    use crate::lint::expanded_source::ExpandedSource;

    struct ReachFixture;

    impl ReachFixture {
        fn unreached(text: &str) -> Vec<String> {
            let source = ExpandedSource::expand(text);
            let program = source.program();
            let candidates = DefinitionReach::candidates(program);
            let mut names: Vec<String> = DefinitionReach::of(program, candidates)
                .unreached_in(program.entry.module)
                .iter()
                .map(|definition| program.bindings.get(definition.binding).name().to_string())
                .collect();
            names.sort();
            names
        }

        fn defined(program: &Program, name: &str) -> BindingId {
            program
                .modules
                .iter()
                .flat_map(|module| module.definitions())
                .map(|(binding, _)| binding)
                .find(|binding| program.bindings.get(*binding).name() == name)
                .expect("the program defines the name")
        }
    }

    #[test]
    fn a_private_definition_read_from_another_project_module_is_reached() {
        let source = ExpandedSource::expand(
            "(use tests/fixtures/lint/modules/doubling twice-of)\n(defn main () (println (twice-of 3)))",
        );
        let program = source.program();
        let double = ReachFixture::defined(program, "double");
        let mut candidates = DefinitionReach::candidates(program);
        candidates.insert(
            double,
            program
                .bindings
                .get(double)
                .kind()
                .defining_module()
                .expect("a global"),
        );
        let reach = DefinitionReach::of(program, candidates);
        let unreached = program
            .modules
            .iter()
            .flat_map(|module| reach.unreached_in(module.id));
        assert!(
            unreached
                .map(|definition| definition.binding)
                .all(|binding| binding != double)
        );
    }

    #[test]
    fn a_cycle_that_no_root_reads_is_unreached() {
        let unreached = ReachFixture::unreached(
            "(defn ping (n) (pong n))\n(defn pong (n) (ping n))\n(defn main () (println 1))",
        );
        assert_eq!(unreached, ["ping", "pong"]);
    }

    #[test]
    fn a_definition_that_main_reads_is_reached() {
        let unreached = ReachFixture::unreached(
            "(defn double (n) (* 2 n))\n(defn main () (println (double 1)))",
        );
        assert!(unreached.is_empty(), "{unreached:?}");
    }
}
