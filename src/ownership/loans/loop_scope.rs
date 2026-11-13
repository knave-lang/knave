use super::control_flow_graph::ControlFlowGraph;
use super::loop_heads::LoopHeads;
use super::point_id::PointId;
use crate::types::TypedBinder;

pub struct LoopScope<'b, 'w> {
    graph: &'b mut ControlFlowGraph,
    heads: &'b mut LoopHeads<'w>,
}

impl<'b, 'w> LoopScope<'b, 'w> {
    pub fn new(graph: &'b mut ControlFlowGraph, heads: &'b mut LoopHeads<'w>) -> Self {
        Self { graph, heads }
    }

    pub fn enter(&mut self, head: PointId, parameters: &'w [TypedBinder]) {
        self.heads.enter(head, parameters);
    }

    pub fn leave(&mut self) {
        self.heads.leave();
    }

    pub fn innermost_parameters(&self) -> Option<&'w [TypedBinder]> {
        self.heads.innermost_parameters()
    }

    pub fn return_to_innermost(&mut self, from: PointId) {
        if let Some(head) = self.heads.innermost_head() {
            self.graph.link(from, head);
        }
    }
}
