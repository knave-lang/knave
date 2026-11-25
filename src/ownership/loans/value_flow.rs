use super::control_flow_graph::ControlFlowGraph;
use super::holdings::Holdings;
use super::point_id::PointId;
use super::value_regions::ValueRegions;
use crate::types::TypedBinder;

pub struct ValueFlow<'b> {
    graph: &'b mut ControlFlowGraph,
    regions: &'b mut ValueRegions,
}

impl<'b> ValueFlow<'b> {
    pub fn new(graph: &'b mut ControlFlowGraph, regions: &'b mut ValueRegions) -> Self {
        Self { graph, regions }
    }

    pub fn define(&mut self, point: PointId, binder: &TypedBinder, held: &Holdings) {
        self.graph.define(point, binder.binding);
        self.regions.flow_into(binder.binding, held);
    }
}
