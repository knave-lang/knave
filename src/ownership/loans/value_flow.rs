use super::body_regions::BodyRegions;
use super::control_flow_graph::ControlFlowGraph;
use super::holdings::Holdings;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use crate::core::BindingId;

pub struct ValueFlow<'b> {
    graph: &'b mut ControlFlowGraph,
    regions: &'b mut BodyRegions,
}

impl<'b> ValueFlow<'b> {
    pub fn new(graph: &'b mut ControlFlowGraph, regions: &'b mut BodyRegions) -> Self {
        Self { graph, regions }
    }

    pub fn define(&mut self, point: PointId, binding: BindingId, held: &Holdings) {
        self.graph.define(point, binding);
        self.regions.flow_into(point, binding, held);
    }

    pub fn store(&mut self, store: RecordedStore) {
        self.regions.store(store);
    }
}
