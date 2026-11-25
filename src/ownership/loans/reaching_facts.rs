use rustc_hash::FxHashMap;

use super::fact_row::FactRow;
use super::flow_id::FlowId;
use super::forward_facts::{FactTransfer, ForwardFacts};
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_flow::RecordedFlow;
use crate::core::BindingId;

#[derive(Default)]
pub struct ReachingFacts {
    flows: Vec<RecordedFlow>,
    by_binding: FxHashMap<BindingId, Vec<FlowId>>,
    reach: ForwardFacts,
}

impl ReachingFacts {
    pub fn of(graph: &PointGraph, flows: Vec<RecordedFlow>) -> Self {
        let mut by_binding: FxHashMap<BindingId, Vec<FlowId>> = FxHashMap::default();
        for (index, flow) in flows.iter().enumerate() {
            by_binding
                .entry(flow.binding)
                .or_default()
                .push(FlowId::new(index));
        }
        let definitions = Definitions::of(&flows, &by_binding);
        let reach = ForwardFacts::solved(graph, flows.len(), &definitions);
        Self {
            flows,
            by_binding,
            reach,
        }
    }

    pub fn reaching(
        &self,
        binding: BindingId,
        point: PointId,
    ) -> impl Iterator<Item = FlowId> + '_ {
        self.by_binding
            .get(&binding)
            .into_iter()
            .flatten()
            .copied()
            .filter(move |flow| self.reach.entering(point, flow.index()))
    }

    pub fn recorded(&self) -> impl Iterator<Item = (FlowId, &RecordedFlow)> {
        self.flows
            .iter()
            .enumerate()
            .map(|(index, recorded)| (FlowId::new(index), recorded))
    }

    pub fn count(&self) -> usize {
        self.flows.len()
    }

    pub fn bindings(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.by_binding.keys().copied()
    }
}

struct Definitions {
    generated: FxHashMap<PointId, Vec<FlowId>>,
    killed: FxHashMap<PointId, Vec<FlowId>>,
}

impl Definitions {
    fn of(flows: &[RecordedFlow], by_binding: &FxHashMap<BindingId, Vec<FlowId>>) -> Self {
        let mut generated: FxHashMap<PointId, Vec<FlowId>> = FxHashMap::default();
        for (index, flow) in flows.iter().enumerate() {
            generated
                .entry(flow.point)
                .or_default()
                .push(FlowId::new(index));
        }
        let killed = generated
            .iter()
            .map(|(point, defined)| (*point, Self::killed_by(flows, by_binding, defined)))
            .collect();
        Self { generated, killed }
    }

    fn killed_by(
        flows: &[RecordedFlow],
        by_binding: &FxHashMap<BindingId, Vec<FlowId>>,
        defined: &[FlowId],
    ) -> Vec<FlowId> {
        defined
            .iter()
            .flat_map(|flow| &by_binding[&flows[flow.index()].binding])
            .filter(|other| !defined.contains(*other))
            .copied()
            .collect()
    }
}

impl FactTransfer for Definitions {
    fn transfer(&self, point: PointId, facts: &mut FactRow) {
        for flow in self.killed.get(&point).into_iter().flatten() {
            facts.clear(flow.index());
        }
        for flow in self.generated.get(&point).into_iter().flatten() {
            facts.set(flow.index());
        }
    }
}
