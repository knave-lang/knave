use rustc_hash::FxHashMap;

use super::node::{NodeId, Source};
use super::slot::Slot;
use super::text_value::TextValue;
use super::transfer::Transfer;
use crate::source::Span;

#[derive(Clone, Copy, Debug)]
pub struct Flow {
    pub source: Source,
    pub target: NodeId,
    pub transfer: Transfer,
    pub span: Span,
}

#[derive(Default)]
pub struct FlowGraph {
    slots: Vec<Slot>,
    ids: FxHashMap<Slot, NodeId>,
    parents: Vec<NodeId>,
    flows: Vec<Flow>,
}

impl FlowGraph {
    pub fn node(&mut self, slot: Slot) -> NodeId {
        if let Some(id) = self.ids.get(&slot) {
            return *id;
        }
        let id = NodeId(self.slots.len());
        self.slots.push(slot.clone());
        self.parents.push(id);
        self.ids.insert(slot, id);
        id
    }

    pub fn receive(&mut self, slot: &Slot, value: &TextValue) {
        let target = self.node(slot.clone());
        for (source, transfer, span) in value.sources() {
            self.flows.push(Flow {
                source: *source,
                target,
                transfer: *transfer,
                span: *span,
            });
        }
        for (path, nested) in value.nested() {
            let inner = self.node(Slot::new(slot.owner.clone(), slot.path.joined(path)));
            self.unite(inner, *nested);
        }
    }

    pub fn unite(&mut self, first: NodeId, second: NodeId) {
        let (first, second) = (self.class(first), self.class(second));
        if first != second {
            self.parents[second.0] = first;
        }
    }

    pub fn class(&self, node: NodeId) -> NodeId {
        let mut current = node;
        while self.parents[current.0] != current {
            current = self.parents[current.0];
        }
        current
    }

    pub fn slot(&self, node: NodeId) -> &Slot {
        &self.slots[node.0]
    }

    pub fn flows(&self) -> &[Flow] {
        &self.flows
    }

    pub fn classes(&self) -> FxHashMap<NodeId, Vec<NodeId>> {
        let mut classes: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
        for node in (0..self.slots.len()).map(NodeId) {
            classes.entry(self.class(node)).or_default().push(node);
        }
        classes
    }
}

#[cfg(test)]
mod tests {
    use super::{FlowGraph, Slot};
    use crate::core::BindingId;
    use crate::ownership::text::slot::SlotOwner;

    struct Locals;

    impl Locals {
        fn slot(index: u32) -> Slot {
            Slot::top(SlotOwner::Local(BindingId::from_index(index)))
        }
    }

    #[test]
    fn a_slot_asked_for_twice_is_one_node() {
        let mut graph = FlowGraph::default();
        assert_eq!(graph.node(Locals::slot(1)), graph.node(Locals::slot(1)));
    }

    #[test]
    fn united_nodes_share_one_class() {
        let mut graph = FlowGraph::default();
        let first = graph.node(Locals::slot(1));
        let second = graph.node(Locals::slot(2));
        graph.unite(first, second);
        graph.unite(second, first);
        assert_eq!(graph.class(first), graph.class(second));
        assert_eq!(graph.classes()[&graph.class(first)].len(), 2);
    }
}
