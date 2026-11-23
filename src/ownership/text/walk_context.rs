use super::flow_graph::FlowGraph;
use super::flow_origin::FlowOrigin;
use super::need::Need;
use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_reads::SlotReads;
use super::text_program::TextProgram;
use super::text_read::TextRead;
use super::text_value::TextValue;

pub struct WalkContext<'w, 'p> {
    program: &'w TextProgram<'p>,
    graph: &'w mut FlowGraph,
    origin: FlowOrigin,
}

impl<'w, 'p> WalkContext<'w, 'p> {
    pub fn new(program: &'w TextProgram<'p>, graph: &'w mut FlowGraph, origin: FlowOrigin) -> Self {
        Self {
            program,
            graph,
            origin,
        }
    }

    pub fn program(&self) -> &'w TextProgram<'p> {
        self.program
    }

    pub fn receive(&mut self, slot: &Slot, value: &TextValue) {
        self.graph.receive(slot, value, self.origin);
    }

    pub fn node(&mut self, slot: Slot) -> NodeId {
        self.graph.node(slot)
    }

    pub fn unite(&mut self, first: NodeId, second: NodeId) {
        self.graph.unite(first, second);
    }

    pub fn require(&mut self, slot: Slot, need: Need) {
        self.graph.require(slot, need);
    }

    pub fn read(&mut self, base: &Slot, read: TextRead<'_>) -> TextValue {
        SlotReads::new(self.graph).read(base, read)
    }
}
