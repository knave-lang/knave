use rustc_hash::FxHashMap;

use super::flow::Flow;
use super::need::Need;
use super::node_id::NodeId;
use super::node_partition::NodePartition;
use super::slot::Slot;
use super::text_value::TextValue;

#[derive(Default)]
pub struct FlowGraph {
    slots: Vec<Slot>,
    ids: FxHashMap<Slot, NodeId>,
    partition: NodePartition,
    flows: Vec<Flow>,
}

impl FlowGraph {
    pub fn node(&mut self, slot: Slot) -> NodeId {
        if let Some(id) = self.ids.get(&slot) {
            return *id;
        }
        let id = self.partition.add();
        self.slots.push(slot.clone());
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
            let inner = self.node(slot.nested(path));
            self.unite(inner, *nested);
        }
    }

    pub fn unite(&mut self, first: NodeId, second: NodeId) {
        self.partition.unite(first, second);
    }

    pub fn require(&mut self, slot: Slot, need: Need) {
        let node = self.node(slot);
        self.partition.require(node, need);
    }

    pub fn partition(&self) -> &NodePartition {
        &self.partition
    }

    pub fn slot(&self, node: NodeId) -> &Slot {
        &self.slots[node.0]
    }

    pub fn flows(&self) -> &[Flow] {
        &self.flows
    }
}

#[cfg(test)]
mod tests {
    use super::{FlowGraph, Slot};
    use crate::core::BindingId;
    use crate::ownership::text::slot_owner::SlotOwner;

    struct Locals;

    impl Locals {
        fn slot(index: u32) -> Slot {
            Slot::from(SlotOwner::Local(BindingId::from_index(index)))
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
        let partition = graph.partition();
        assert_eq!(partition.class(first), partition.class(second));
        assert_eq!(partition.classes()[&partition.class(first)].len(), 2);
    }
}
