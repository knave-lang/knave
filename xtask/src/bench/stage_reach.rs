use std::collections::{BTreeMap, BTreeSet};

use super::allocator_share::AllocatorShare;
use super::call_graph::CallGraph;
use super::stage_entry::StageEntry;

pub struct StageReach {
    stage_of: BTreeMap<String, String>,
}

impl StageReach {
    pub fn of(graph: &CallGraph<'_>, entries: &[StageEntry]) -> Self {
        let outside = Self::outside(graph, entries);
        let stages_by_function = Self::stages_by_function(graph, entries, &outside);
        Self {
            stage_of: Self::reached_by_one_stage(stages_by_function),
        }
    }

    pub fn allocator_charges<'g>(
        &'g self,
        graph: &'g CallGraph<'g>,
    ) -> impl Iterator<Item = (&'g str, u64)> + 'g {
        graph
            .calls()
            .filter(|(_, callee, _)| AllocatorShare::is_allocator_call(callee))
            .filter_map(|(caller, _, cost)| Some((self.stage_of.get(caller)?.as_str(), cost)))
    }

    fn outside<'g>(graph: &'g CallGraph<'_>, entries: &[StageEntry]) -> BTreeSet<&'g str> {
        graph.reached_from(graph.roots(), |function| Self::stops_at(function, entries))
    }

    fn stages_by_function<'g>(
        graph: &'g CallGraph<'_>,
        entries: &'g [StageEntry],
        outside: &BTreeSet<&str>,
    ) -> BTreeMap<&'g str, BTreeSet<&'g str>> {
        let mut stages_by_function: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for entry in entries {
            let reached = graph.reached_from([entry.function.as_str()], |function| {
                Self::stops_at(function, entries)
            });
            for function in reached
                .into_iter()
                .filter(|function| !outside.contains(function))
            {
                stages_by_function
                    .entry(function)
                    .or_default()
                    .insert(&entry.stage);
            }
        }
        stages_by_function
    }

    fn reached_by_one_stage(
        stages_by_function: BTreeMap<&str, BTreeSet<&str>>,
    ) -> BTreeMap<String, String> {
        stages_by_function
            .into_iter()
            .filter_map(|(function, stages)| {
                let stage = stages.first()?;
                (stages.len() == 1).then(|| (function.to_string(), stage.to_string()))
            })
            .collect()
    }

    fn stops_at(function: &str, entries: &[StageEntry]) -> bool {
        AllocatorShare::is_allocator_call(function)
            || entries.iter().any(|entry| entry.function == function)
    }
}
