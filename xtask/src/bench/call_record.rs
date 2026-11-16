use std::collections::{BTreeMap, BTreeSet};

use super::call::Call;
use super::call_graph::CallGraph;
use super::stage_costs::{StageCostProblem, StageCosts};
use super::stage_entry::StageEntry;
use super::stage_reach::StageReach;

#[derive(Default)]
pub struct CallRecord<'text> {
    costs: StageCosts,
    graph: CallGraph<'text>,
    entered: BTreeSet<&'text str>,
}

impl<'text> CallRecord<'text> {
    pub fn record(&mut self, call: Call<'text>, stage: Option<String>) {
        self.graph.add_call(call);
        if let Some(stage) = stage {
            self.costs.add_inclusive(stage, call.cost);
            self.entered.insert(call.callee);
        }
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
