use rustc_hash::{FxHashMap, FxHashSet};

use super::flow_graph::FlowGraph;
use super::node_id::NodeId;
use super::slot_classes::SlotClasses;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::transfer::Transfer;
use crate::core::BindingId;

#[derive(Default)]
pub struct CallResults {
    results: FxHashMap<NodeId, NodeId>,
    reaching: FxHashMap<NodeId, FxHashSet<BindingId>>,
}

impl CallResults {
    pub fn of(graph: &FlowGraph, classes: &SlotClasses<'_>) -> Self {
        let mut results = FxHashMap::default();
        for flow in graph.flows() {
            let Source::Slot(source) = flow.source else {
                continue;
            };
            let per_call = matches!(graph.slot(flow.target).owner(), SlotOwner::CallResult(_));
            if per_call && matches!(graph.slot(source).owner(), SlotOwner::Result(_)) {
                results.insert(classes.class(flow.target), classes.class(source));
            }
        }
        let reaching = results
            .values()
            .map(|result| (*result, Self::parameters_reaching(graph, classes, *result)))
            .collect();
        Self { results, reaching }
    }

    pub fn result_of(&self, per_call: NodeId) -> Option<NodeId> {
        self.results.get(&per_call).copied()
    }

    pub fn reaches(&self, result: NodeId, parameter: BindingId) -> bool {
        self.reaching
            .get(&result)
            .is_some_and(|parameters| parameters.contains(&parameter))
    }

    fn parameters_reaching(
        graph: &FlowGraph,
        classes: &SlotClasses<'_>,
        result: NodeId,
    ) -> FxHashSet<BindingId> {
        let mut seen = FxHashSet::default();
        let mut pending = vec![result];
        while let Some(class) = pending.pop() {
            if !seen.insert(class) {
                continue;
            }
            for flow in graph.flows() {
                if let (Source::Slot(source), Transfer::Move | Transfer::Refer | Transfer::Reborrow) =
                    (flow.source, flow.transfer)
                    && classes.class(flow.target) == class
                {
                    pending.push(classes.class(source));
                }
            }
        }
        graph
            .partition()
            .classes()
            .into_iter()
            .filter(|(class, _)| seen.contains(class))
            .flat_map(|(_, members)| members)
            .filter(|node| classes.is_read_only_parameter(*node))
            .filter_map(|node| graph.slot(node).parameter())
            .collect()
    }
}
