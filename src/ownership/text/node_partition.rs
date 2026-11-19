use rustc_hash::FxHashMap;

use super::need::Need;
use super::node_id::NodeId;

#[derive(Default)]
pub struct NodePartition {
    parents: Vec<NodeId>,
    needs: FxHashMap<NodeId, Need>,
}

impl NodePartition {
    pub fn add(&mut self) -> NodeId {
        let node = NodeId(self.parents.len());
        self.parents.push(node);
        node
    }

    pub fn unite(&mut self, first: NodeId, second: NodeId) {
        let (first, second) = (self.class(first), self.class(second));
        if first != second {
            self.parents[second.0] = first;
            self.move_need(second, first);
        }
    }

    pub fn require(&mut self, node: NodeId, need: Need) {
        let class = self.class(node);
        let required = self
            .needs
            .get(&class)
            .map_or(need, |held| held.stronger(need));
        self.needs.insert(class, required);
    }

    pub fn class(&self, node: NodeId) -> NodeId {
        let mut current = node;
        while self.parents[current.0] != current {
            current = self.parents[current.0];
        }
        current
    }

    pub fn need(&self, node: NodeId) -> Option<Need> {
        self.needs.get(&self.class(node)).copied()
    }

    pub fn classes(&self) -> FxHashMap<NodeId, Vec<NodeId>> {
        let mut classes: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
        for node in (0..self.parents.len()).map(NodeId) {
            classes.entry(self.class(node)).or_default().push(node);
        }
        classes
    }

    fn move_need(&mut self, from: NodeId, to: NodeId) {
        if let Some(need) = self.needs.remove(&from) {
            self.require(to, need);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Need, NodePartition};

    #[test]
    fn a_need_follows_its_node_into_a_united_class() {
        let mut partition = NodePartition::default();
        let (first, second) = (partition.add(), partition.add());
        partition.require(second, Need::View);
        partition.unite(first, second);
        assert_eq!(partition.need(first), Some(Need::View));
    }

    #[test]
    fn uniting_two_needs_keeps_the_stronger() {
        let mut partition = NodePartition::default();
        let (first, second) = (partition.add(), partition.add());
        partition.require(first, Need::OwnedOrMixed);
        partition.require(second, Need::Owned);
        partition.unite(first, second);
        assert_eq!(partition.need(second), Some(Need::Owned));
    }
}
