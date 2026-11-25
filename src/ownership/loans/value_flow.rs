use super::body_regions::BodyRegions;
use super::control_flow_graph::ControlFlowGraph;
use super::holdings::Holdings;
use super::new_value::NewValue;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use crate::core::BindingId;
use crate::types::TypedBinder;

pub struct ValueFlow<'b> {
    graph: &'b mut ControlFlowGraph,
    regions: &'b mut BodyRegions,
}

impl<'b> ValueFlow<'b> {
    pub fn new(graph: &'b mut ControlFlowGraph, regions: &'b mut BodyRegions) -> Self {
        Self { graph, regions }
    }

    pub fn define(&mut self, point: PointId, binder: &TypedBinder, held: &Holdings) {
        self.graph.define(point, NewValue::Bound(binder.binding));
        self.regions.flow_into(binder.binding, held);
    }

    pub fn store(&mut self, store: RecordedStore) {
        self.regions.store(store);
    }

    pub fn replace(&mut self, point: PointId, binding: BindingId) {
        self.graph.define(point, NewValue::Replaced(binding));
    }
}
