use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::graph::DependencyGraph;
use crate::core::BindingId;

#[derive(Default)]
pub(super) struct DefinitionOutcomes {
    failed: FxHashSet<BindingId>,
    after_errors: FxHashSet<BindingId>,
    unreached: Rc<FxHashSet<BindingId>>,
}

impl DefinitionOutcomes {
    pub(super) fn new(unreached: Rc<FxHashSet<BindingId>>) -> Self {
        Self {
            unreached,
            ..Self::default()
        }
    }

    pub(super) fn fail(&mut self, bindings: impl IntoIterator<Item = BindingId>) {
        self.failed.extend(bindings);
    }

    pub(super) fn is_blocked(&self, dependencies: &DependencyGraph, group: &[BindingId]) -> bool {
        self.has_failed_member(group) || dependencies.directly_depends_on_any(group, &self.failed)
    }

    pub(super) fn finish_group(
        &mut self,
        dependencies: &DependencyGraph,
        group: &[BindingId],
        reported_errors: usize,
    ) {
        self.note_group_after_errors(dependencies, group, reported_errors);
        self.fail_group_when_a_member_failed(group);
    }

    pub(super) fn follows_reported_error(&self, binding: BindingId) -> bool {
        self.after_errors.contains(&binding)
    }

    pub(super) fn is_unreached(&self, binding: BindingId) -> bool {
        self.unreached.contains(&binding)
    }

    fn note_group_after_errors(
        &mut self,
        dependencies: &DependencyGraph,
        group: &[BindingId],
        reported_errors: usize,
    ) {
        if reported_errors > 0 || dependencies.directly_depends_on_any(group, &self.after_errors) {
            self.after_errors.extend(group);
        }
    }

    fn fail_group_when_a_member_failed(&mut self, group: &[BindingId]) {
        if self.has_failed_member(group) {
            self.failed.extend(group);
        }
    }

    fn has_failed_member(&self, group: &[BindingId]) -> bool {
        group.iter().any(|binding| self.failed.contains(binding))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use rustc_hash::FxHashSet;

    use super::DefinitionOutcomes;
    use crate::core::BindingId;
    use crate::types::{DefinitionGraph, TypedSource};

    struct CallChain {
        source: TypedSource,
        outcomes: DefinitionOutcomes,
    }

    impl CallChain {
        const SOURCE: &str = "(defn last-step () 1)\n\
                              (defn middle-step () (last-step))\n\
                              (defn first-step () (middle-step))\n";
        const NO_ERRORS: usize = 0;
        const ONE_ERROR: usize = 1;

        fn new() -> Self {
            Self {
                source: TypedSource::expand(Self::SOURCE),
                outcomes: DefinitionOutcomes::default(),
            }
        }

        fn last(&self) -> BindingId {
            self.named("last-step")
        }

        fn middle(&self) -> BindingId {
            self.named("middle-step")
        }

        fn first(&self) -> BindingId {
            self.named("first-step")
        }

        fn named(&self, name: &str) -> BindingId {
            let bindings = &self.source.program().bindings;
            bindings
                .ids()
                .find(|binding| bindings.get(*binding).name() == name)
                .expect("the source defines the name")
        }

        fn is_blocked(&self, binding: BindingId) -> bool {
            let graph = DefinitionGraph::of(self.source.program());
            self.outcomes.is_blocked(graph.dependencies(), &[binding])
        }

        fn finish(&mut self, group: &[BindingId], reported_errors: usize) {
            let graph = DefinitionGraph::of(self.source.program());
            self.outcomes
                .finish_group(graph.dependencies(), group, reported_errors);
        }
    }

    #[test]
    fn a_failed_definition_is_blocked() {
        let mut chain = CallChain::new();
        chain.outcomes.fail([chain.last()]);
        assert!(chain.is_blocked(chain.last()));
    }

    #[test]
    fn a_direct_caller_of_a_failed_definition_is_blocked() {
        let mut chain = CallChain::new();
        chain.outcomes.fail([chain.last()]);
        assert!(chain.is_blocked(chain.middle()));
    }

    #[test]
    fn a_caller_two_calls_from_a_failure_is_not_blocked() {
        let mut chain = CallChain::new();
        chain.outcomes.fail([chain.last()]);
        assert!(!chain.is_blocked(chain.first()));
    }

    #[test]
    fn a_group_with_one_failed_member_fails_whole() {
        let mut chain = CallChain::new();
        chain.outcomes.fail([chain.last()]);
        chain.finish(&[chain.last(), chain.first()], CallChain::NO_ERRORS);
        assert!(chain.is_blocked(chain.first()));
    }

    #[test]
    fn a_group_without_a_failed_member_stays_checkable() {
        let mut chain = CallChain::new();
        chain.finish(&[chain.first()], CallChain::NO_ERRORS);
        assert!(!chain.is_blocked(chain.first()));
    }

    #[test]
    fn a_group_that_reported_an_error_follows_it() {
        let mut chain = CallChain::new();
        chain.finish(&[chain.last()], CallChain::ONE_ERROR);
        assert!(chain.outcomes.follows_reported_error(chain.last()));
    }

    #[test]
    fn a_clean_group_does_not_follow_an_error() {
        let mut chain = CallChain::new();
        chain.finish(&[chain.last()], CallChain::NO_ERRORS);
        assert!(!chain.outcomes.follows_reported_error(chain.last()));
    }

    #[test]
    fn a_direct_caller_of_a_group_after_errors_follows_them() {
        let mut chain = CallChain::new();
        chain.finish(&[chain.last()], CallChain::ONE_ERROR);
        chain.finish(&[chain.middle()], CallChain::NO_ERRORS);
        assert!(chain.outcomes.follows_reported_error(chain.middle()));
    }

    #[test]
    fn a_definition_the_graph_left_unreached_is_unreached() {
        let chain = CallChain::new();
        let outcomes = DefinitionOutcomes::new(Rc::new(FxHashSet::from_iter([chain.last()])));
        assert!(outcomes.is_unreached(chain.last()));
        assert!(!outcomes.is_unreached(chain.first()));
    }
}
