use rustc_hash::{FxHashMap, FxHashSet};

use super::written_bodies::WrittenBodies;
use crate::core::{BindingId, References};

pub struct DependencyGraph {
    references: FxHashMap<BindingId, References>,
    edges: FxHashMap<BindingId, Vec<BindingId>>,
    always_emitted: FxHashSet<BindingId>,
    compile_time_only: FxHashSet<BindingId>,
    macro_helpers: FxHashSet<BindingId>,
}

impl DependencyGraph {
    pub(super) fn build(
        walked: FxHashMap<BindingId, References>,
        macro_helpers: FxHashSet<BindingId>,
        written: &WrittenBodies,
    ) -> Self {
        let mut edges = FxHashMap::with_capacity_and_hasher(walked.len(), Default::default());
        let mut direct_compile_time = FxHashSet::default();
        for (binding, references) in &walked {
            if references.uses_compile_time_code() {
                direct_compile_time.insert(*binding);
            }
            edges.insert(
                *binding,
                references
                    .globals()
                    .iter()
                    .filter(|global| walked.contains_key(global))
                    .copied()
                    .collect(),
            );
        }
        let mut graph = Self {
            references: walked,
            macro_helpers,
            edges,
            always_emitted: written.iter().collect(),
            compile_time_only: direct_compile_time,
        };
        graph.spread_compile_time();
        graph
    }

    pub fn references(&self, binding: BindingId) -> Option<&References> {
        self.references.get(&binding)
    }

    pub fn is_compile_time_only(&self, binding: BindingId) -> bool {
        self.compile_time_only.contains(&binding)
    }

    pub fn unchecked(&self, mains: &[BindingId]) -> FxHashSet<BindingId> {
        let roots: Vec<BindingId> = mains.iter().chain(&self.always_emitted).copied().collect();
        let runtime = self.reachable_from(&roots);
        let checked_helpers = self.reachable_from(&self.runtime_roots(&roots));
        self.edges
            .keys()
            .filter(|binding| {
                (self.is_compile_time_only(**binding) && !runtime.contains(binding))
                    || (self.macro_helpers.contains(binding) && !checked_helpers.contains(binding))
            })
            .copied()
            .collect()
    }

    fn runtime_roots(&self, mains: &[BindingId]) -> Vec<BindingId> {
        self.edges
            .keys()
            .filter(|binding| {
                !self.is_compile_time_only(**binding) && !self.macro_helpers.contains(binding)
            })
            .chain(mains)
            .copied()
            .collect()
    }

    pub fn directly_depends_on_any(
        &self,
        roots: &[BindingId],
        bindings: &FxHashSet<BindingId>,
    ) -> bool {
        roots
            .iter()
            .filter_map(|root| self.edges.get(root))
            .flatten()
            .any(|dependency| bindings.contains(dependency))
    }

    pub fn reachable_from(&self, roots: &[BindingId]) -> FxHashSet<BindingId> {
        let mut reached: FxHashSet<BindingId> = FxHashSet::default();
        let mut pending: Vec<BindingId> = roots.to_vec();
        while let Some(binding) = pending.pop() {
            if reached.insert(binding) {
                pending.extend(self.edges.get(&binding).into_iter().flatten().copied());
            }
        }
        reached
    }

    pub fn components(&self, order: &[BindingId]) -> Vec<Vec<BindingId>> {
        StronglyConnected::new(&self.edges).run(order)
    }

    fn spread_compile_time(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            for (binding, dependencies) in &self.edges {
                let depends_on_compile_time = dependencies
                    .iter()
                    .any(|dependency| self.compile_time_only.contains(dependency));
                if depends_on_compile_time && !self.compile_time_only.contains(binding) {
                    self.compile_time_only.insert(*binding);
                    changed = true;
                }
            }
        }
    }
}

struct StronglyConnected<'a> {
    edges: &'a FxHashMap<BindingId, Vec<BindingId>>,
    index: FxHashMap<BindingId, usize>,
    low_link: FxHashMap<BindingId, usize>,
    stack: Vec<BindingId>,
    on_stack: FxHashSet<BindingId>,
    components: Vec<Vec<BindingId>>,
}

impl<'a> StronglyConnected<'a> {
    fn new(edges: &'a FxHashMap<BindingId, Vec<BindingId>>) -> Self {
        Self {
            edges,
            index: FxHashMap::default(),
            low_link: FxHashMap::default(),
            stack: Vec::new(),
            on_stack: FxHashSet::default(),
            components: Vec::new(),
        }
    }

    fn run(mut self, order: &[BindingId]) -> Vec<Vec<BindingId>> {
        for binding in order {
            if !self.index.contains_key(binding) {
                self.visit(*binding);
            }
        }
        self.components
    }

    fn visit(&mut self, binding: BindingId) {
        let position = self.index.len();
        self.index.insert(binding, position);
        self.low_link.insert(binding, position);
        self.stack.push(binding);
        self.on_stack.insert(binding);
        for dependency in self.edges.get(&binding).cloned().unwrap_or_default() {
            self.visit_dependency(binding, dependency);
        }
        if self.low_link[&binding] == self.index[&binding] {
            self.pop_component(binding);
        }
    }

    fn visit_dependency(&mut self, binding: BindingId, dependency: BindingId) {
        if !self.index.contains_key(&dependency) {
            self.visit(dependency);
            let lowest = self.low_link[&binding].min(self.low_link[&dependency]);
            self.low_link.insert(binding, lowest);
        } else if self.on_stack.contains(&dependency) {
            let lowest = self.low_link[&binding].min(self.index[&dependency]);
            self.low_link.insert(binding, lowest);
        }
    }

    fn pop_component(&mut self, root: BindingId) {
        let mut component = Vec::new();
        while let Some(member) = self.stack.pop() {
            self.on_stack.remove(&member);
            component.push(member);
            if member == root {
                break;
            }
        }
        component.reverse();
        self.components.push(component);
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::{FxHashMap, FxHashSet};

    use super::DependencyGraph;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId};

    struct CallGraph {
        bindings: BindingArena,
        edges: FxHashMap<BindingId, Vec<BindingId>>,
        always_emitted: FxHashSet<BindingId>,
        compile_time_code: FxHashSet<BindingId>,
        macro_helpers: FxHashSet<BindingId>,
    }

    impl CallGraph {
        fn new() -> Self {
            Self {
                bindings: BindingArena::default(),
                edges: FxHashMap::default(),
                always_emitted: FxHashSet::default(),
                compile_time_code: FxHashSet::default(),
                macro_helpers: FxHashSet::default(),
            }
        }

        fn compile_time_function(&mut self, name: &str, callees: &[BindingId]) -> BindingId {
            let binding = self.function(name, callees);
            self.compile_time_code.insert(binding);
            binding
        }

        fn written_body(&mut self, name: &str, callees: &[BindingId]) -> BindingId {
            let binding = self.compile_time_function(name, callees);
            self.always_emitted.insert(binding);
            binding
        }

        fn macro_helper(&mut self, name: &str) -> BindingId {
            let binding = self.function(name, &[]);
            self.macro_helpers.insert(binding);
            binding
        }

        fn function(&mut self, name: &str, callees: &[BindingId]) -> BindingId {
            let kind = BindingKind::Global(ModuleId::new(0));
            let binding = self.bindings.add(BindingInfo::new(name.into(), kind, None));
            self.edges.insert(binding, callees.to_vec());
            binding
        }

        fn dependencies(self) -> DependencyGraph {
            DependencyGraph {
                references: FxHashMap::default(),
                edges: self.edges,
                always_emitted: self.always_emitted,
                compile_time_only: self.compile_time_code,
                macro_helpers: self.macro_helpers,
            }
        }
    }

    #[test]
    fn a_failed_callee_is_a_direct_dependency() {
        let mut graph = CallGraph::new();
        let last = graph.function("last-step", &[]);
        let caller = graph.function("middle-step", &[last]);
        let failed = FxHashSet::from_iter([last]);
        assert!(
            graph
                .dependencies()
                .directly_depends_on_any(&[caller], &failed)
        );
    }

    #[test]
    fn a_failure_two_calls_away_is_not_a_direct_dependency() {
        let mut graph = CallGraph::new();
        let last = graph.function("last-step", &[]);
        let middle = graph.function("middle-step", &[last]);
        let first = graph.function("first-step", &[middle]);
        let failed = FxHashSet::from_iter([last]);
        assert!(
            !graph
                .dependencies()
                .directly_depends_on_any(&[first], &failed)
        );
    }

    #[test]
    fn an_unreached_compile_time_body_that_emit_writes_is_checked() {
        let mut graph = CallGraph::new();
        let body = graph.written_body("greet-method", &[]);
        let main = graph.function("main", &[]);
        assert!(!graph.dependencies().unchecked(&[main]).contains(&body));
    }

    #[test]
    fn compile_time_code_that_a_written_body_calls_is_checked() {
        let mut graph = CallGraph::new();
        let helper = graph.compile_time_function("quoted-name", &[]);
        graph.written_body("greet-method", &[helper]);
        let main = graph.function("main", &[]);
        assert!(!graph.dependencies().unchecked(&[main]).contains(&helper));
    }

    #[test]
    fn a_macro_helper_that_only_a_written_body_calls_is_checked() {
        let mut graph = CallGraph::new();
        let helper = graph.macro_helper("record-name");
        graph.written_body("greet-method", &[helper]);
        let main = graph.function("main", &[]);
        assert!(!graph.dependencies().unchecked(&[main]).contains(&helper));
    }

    #[test]
    fn a_macro_helper_that_only_compile_time_code_calls_stays_unchecked() {
        let mut graph = CallGraph::new();
        let helper = graph.macro_helper("record-name");
        graph.compile_time_function("expansion", &[helper]);
        let main = graph.function("main", &[]);
        assert!(graph.dependencies().unchecked(&[main]).contains(&helper));
    }

    #[test]
    fn a_macro_helper_that_runtime_code_calls_is_checked() {
        let mut graph = CallGraph::new();
        let helper = graph.macro_helper("record-name");
        graph.function("unused-caller", &[helper]);
        let main = graph.function("main", &[]);
        assert!(!graph.dependencies().unchecked(&[main]).contains(&helper));
    }

    #[test]
    fn a_macro_helper_that_main_reaches_through_compile_time_code_is_checked() {
        let mut graph = CallGraph::new();
        let helper = graph.macro_helper("record-name");
        let expansion = graph.compile_time_function("expansion", &[helper]);
        let main = graph.compile_time_function("main", &[expansion]);
        assert!(!graph.dependencies().unchecked(&[main]).contains(&helper));
    }

    #[test]
    fn compile_time_code_outside_main_stays_unchecked_when_runtime_code_calls_it() {
        let mut graph = CallGraph::new();
        let expansion = graph.compile_time_function("expansion", &[]);
        graph.function("unused-caller", &[expansion]);
        let main = graph.function("main", &[]);
        assert!(graph.dependencies().unchecked(&[main]).contains(&expansion));
    }
}
