use std::collections::{BTreeMap, BTreeSet};

use super::call::Call;
use super::call_graph::CallGraph;
use super::collection_toggles::CollectionToggles;
use super::stage_costs::{StageCostProblem, StageCosts};
use super::stage_entry::StageEntry;
use super::stage_reach::StageReach;

#[derive(Default)]
pub struct CallRecord<'text> {
    costs: StageCosts,
    graph: CallGraph<'text>,
    entered: BTreeSet<&'text str>,
    thread_entry_cost: u64,
}

impl<'text> CallRecord<'text> {
    pub fn record(&mut self, call: Call<'text>, stage: Option<String>) {
        self.graph.add_call(call);
        if call.callee == CollectionToggles::THREAD_ENTRY {
            self.thread_entry_cost += call.cost;
        }
        if let Some(stage) = stage {
            self.costs.add_inclusive(stage, call.cost);
            self.entered.insert(call.callee);
        }
    }

    pub fn collected_nothing_in_a_thread_entry(&self) -> bool {
        self.thread_entry_cost == 0
    }

    pub fn entered(&self) -> &BTreeSet<&'text str> {
        &self.entered
    }

    pub fn into_rows(
        mut self,
        entries: &[StageEntry],
    ) -> Result<BTreeMap<String, u64>, StageCostProblem> {
        let reach = StageReach::of(&self.graph, entries);
        for (stage, cost) in reach.allocator_charges(&self.graph) {
            self.costs.add_allocator(stage.to_string(), cost);
        }
        self.costs.into_rows()
    }
}
