use rustc_hash::{FxHashMap, FxHashSet};

use super::flow_graph::FlowGraph;
use super::node::{NodeId, Source};
use super::slot::Need;
use super::transfer::Transfer;
use crate::core::BindingId;

pub struct StoredParameters<'g> {
    graph: &'g FlowGraph,
    classes: &'g FxHashMap<NodeId, Vec<NodeId>>,
    moves: FxHashMap<NodeId, Vec<NodeId>>,
}

impl<'g> StoredParameters<'g> {
    pub fn new(graph: &'g FlowGraph, classes: &'g FxHashMap<NodeId, Vec<NodeId>>) -> Self {
        let mut moves: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
        for flow in graph.flows() {
            if let (Source::Slot(source), Transfer::Move) = (flow.source, flow.transfer) {
                moves
                    .entry(graph.class(source))
                    .or_default()
                    .push(graph.class(flow.target));
            }
        }
        Self {
            graph,
            classes,
            moves,
        }
    }

    pub fn find(&self) -> FxHashSet<BindingId> {
        let mut stored = FxHashSet::default();
        loop {
            let found = self.newly_stored(&stored);
            if found.is_empty() {
                return stored;
            }
            stored.extend(found);
        }
    }

    fn newly_stored(&self, stored: &FxHashSet<BindingId>) -> Vec<BindingId> {
        self.parameters()
            .filter(|(binding, class)| {
                !stored.contains(binding) && self.reaches_storage(*class, stored)
            })
            .map(|(binding, _)| binding)
            .collect()
    }

    fn parameters(&self) -> impl Iterator<Item = (BindingId, NodeId)> + '_ {
        self.classes.iter().flat_map(|(class, members)| {
            members
                .iter()
                .filter_map(|member| self.graph.slot(*member).parameter())
                .map(|binding| (binding, *class))
        })
    }

    fn reaches_storage(&self, start: NodeId, stored: &FxHashSet<BindingId>) -> bool {
        let mut seen = FxHashSet::default();
        let mut pending: Vec<NodeId> = self.moves.get(&start).cloned().unwrap_or_default();
        while let Some(class) = pending.pop() {
            if self.stores(class, stored) {
                return true;
            }
            if seen.insert(class) {
                pending.extend(self.moves.get(&class).into_iter().flatten());
            }
        }
        false
    }

    fn stores(&self, class: NodeId, stored: &FxHashSet<BindingId>) -> bool {
        let members = &self.classes[&class];
        let written_view = members
            .iter()
            .any(|member| self.graph.slot(*member).need() == Some(Need::View));
        !written_view
            && members.iter().any(|member| {
                let slot = self.graph.slot(*member);
                slot.is_stored_place()
                    || slot.need() == Some(Need::Owned)
                    || slot
                        .parameter()
                        .is_some_and(|binding| stored.contains(&binding))
            })
    }
}
