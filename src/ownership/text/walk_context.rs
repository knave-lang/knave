use super::flow_graph::FlowGraph;
use super::slot_listing::SlotListing;
use super::text_program::TextProgram;

pub struct WalkContext<'w, 'p> {
    program: &'w TextProgram<'p>,
    graph: FlowGraph,
    listing: SlotListing,
}

impl<'w, 'p> WalkContext<'w, 'p> {
    pub fn new(program: &'w TextProgram<'p>) -> Self {
        Self {
            program,
            graph: FlowGraph::default(),
            listing: SlotListing::default(),
        }
    }

    pub fn program(&self) -> &'w TextProgram<'p> {
        self.program
    }

    pub fn graph(&mut self) -> &mut FlowGraph {
        &mut self.graph
    }

    pub fn listing(&mut self) -> &mut SlotListing {
        &mut self.listing
    }

    pub fn finished(self) -> (FlowGraph, SlotListing) {
        (self.graph, self.listing)
    }
}
