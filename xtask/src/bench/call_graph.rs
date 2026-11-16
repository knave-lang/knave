use std::collections::{BTreeMap, BTreeSet};

use super::call::Call;

#[derive(Default)]
pub struct CallGraph<'text> {
    callees: BTreeMap<&'text str, BTreeMap<&'text str, u64>>,
    called: BTreeSet<&'text str>,
}

impl<'text> CallGraph<'text> {
    pub fn add_call(&mut self, call: Call<'text>) {
        *self
            .callees
            .entry(call.caller)
            .or_default()
            .entry(call.callee)
            .or_default() += call.cost;
        self.called.insert(call.callee);
    }

    pub fn calls(&self) -> impl Iterator<Item = (&'text str, &'text str, u64)> + '_ {
        self.callees.iter().flat_map(|(caller, callees)| {
            callees
                .iter()
                .map(|(callee, cost)| (*caller, *callee, *cost))
        })
    }

    pub fn roots(&self) -> impl Iterator<Item = &'text str> + '_ {
        self.callees
            .keys()
            .copied()
            .filter(|caller| !self.called.contains(caller))
    }

    pub fn reached_from<'a>(
        &'a self,
        starts: impl IntoIterator<Item = &'a str>,
        stops_at: impl Fn(&str) -> bool,
    ) -> BTreeSet<&'a str> {
        let mut reached = BTreeSet::new();
        let mut pending: Vec<&'a str> = starts.into_iter().collect();
        while let Some(function) = pending.pop() {
            if !reached.insert(function) {
                continue;
            }
            let callees = self
                .callees
                .get(function)
                .into_iter()
                .flat_map(BTreeMap::keys);
            pending.extend(callees.copied().filter(|callee| !stops_at(callee)));
        }
        reached
    }
}

#[cfg(test)]
mod tests {
    use super::CallGraph;
    use crate::bench::call::Call;

    struct GraphFixture;

    impl GraphFixture {
        fn call(caller: &'static str, callee: &'static str, cost: u64) -> Call<'static> {
            Call {
                caller,
                callee,
                cost,
            }
        }

        fn graph() -> CallGraph<'static> {
            let mut graph = CallGraph::default();
            graph.add_call(GraphFixture::call("main", "parse", 10));
            graph.add_call(GraphFixture::call("parse", "parse", 4));
            graph.add_call(GraphFixture::call("parse", "__rustc::__rust_alloc", 3));
            graph.add_call(GraphFixture::call("main", "print", 2));
            graph.add_call(GraphFixture::call("print", "__rustc::__rust_alloc", 1));
            graph
        }
    }

    #[test]
    fn reaches_through_a_recursive_call_once() {
        let graph = GraphFixture::graph();
        let reached = graph.reached_from(["main"], |_| false);
        assert_eq!(
            reached.into_iter().collect::<Vec<_>>(),
            ["__rustc::__rust_alloc", "main", "parse", "print"]
        );
    }

    #[test]
    fn stops_at_the_functions_it_is_told_to() {
        let graph = GraphFixture::graph();
        let reached = graph.reached_from(["main"], |function| function == "print");
        assert!(!reached.contains("print"));
        assert!(reached.contains("parse"));
    }

    #[test]
    fn sums_the_cost_of_each_call() {
        let mut graph = GraphFixture::graph();
        graph.add_call(GraphFixture::call("parse", "__rustc::__rust_alloc", 5));
        let calls: Vec<_> = graph.calls().collect();
        assert_eq!(
            calls,
            [
                ("main", "parse", 10),
                ("main", "print", 2),
                ("parse", "__rustc::__rust_alloc", 8),
                ("parse", "parse", 4),
                ("print", "__rustc::__rust_alloc", 1),
            ]
        );
    }

    #[test]
    fn roots_are_the_functions_that_nothing_calls() {
        let graph = GraphFixture::graph();
        assert_eq!(graph.roots().collect::<Vec<_>>(), ["main"]);
    }
}
