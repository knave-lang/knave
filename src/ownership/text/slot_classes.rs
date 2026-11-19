use rustc_hash::{FxHashMap, FxHashSet};

use super::class_role::ClassRole;
use super::flow::Flow;
use super::flow_graph::FlowGraph;
use super::need::Need;
use super::node_id::NodeId;
use super::slot::Slot;
use super::source::Source;
use super::stored_parameters::StoredParameters;
use crate::core::BindingId;

pub struct SlotClasses<'g> {
    graph: &'g FlowGraph,
    members: FxHashMap<NodeId, Vec<NodeId>>,
    stored: FxHashSet<BindingId>,
    variables: FxHashSet<NodeId>,
}

impl<'g> SlotClasses<'g> {
    pub fn of(graph: &'g FlowGraph) -> Self {
        let members = graph.partition().classes();
        let stored = StoredParameters::new(graph, &members).find();
        let variables = Self::variable_classes(graph, &members);
        Self {
            graph,
            members,
            stored,
            variables,
        }
    }

    pub fn roles(&self) -> Vec<(NodeId, ClassRole)> {
        let tied = self.tied_variables();
        self.members
            .keys()
            .map(|class| (*class, self.role(*class, tied.get(class).copied())))
            .collect()
    }

    pub fn flows(&self) -> &'g [Flow] {
        self.graph.flows()
    }

    pub fn class(&self, node: NodeId) -> NodeId {
        self.graph.partition().class(node)
    }

    pub fn need(&self, class: NodeId) -> Option<Need> {
        self.graph.partition().need(class)
    }

    pub fn is_variable(&self, class: NodeId) -> bool {
        self.variables.contains(&class)
    }

    pub fn is_read_only_parameter(&self, node: NodeId) -> bool {
        self.graph
            .slot(node)
            .parameter()
            .is_some_and(|binding| !self.stored.contains(&binding))
    }

    fn variable_classes(
        graph: &FlowGraph,
        members: &FxHashMap<NodeId, Vec<NodeId>>,
    ) -> FxHashSet<NodeId> {
        members
            .iter()
            .filter(|(_, members)| {
                members
                    .iter()
                    .any(|member| graph.slot(*member).is_variable())
            })
            .map(|(class, _)| *class)
            .collect()
    }

    fn tied_variables(&self) -> FxHashMap<NodeId, NodeId> {
        self.flows()
            .iter()
            .filter_map(|flow| match flow.source {
                Source::Slot(node) if self.is_read_only_parameter(node) => {
                    Some((self.class(node), self.class(flow.target)))
                }
                _ => None,
            })
            .filter(|(_, target)| self.is_variable(*target))
            .collect()
    }

    fn role(&self, class: NodeId, tied: Option<NodeId>) -> ClassRole {
        let members: Vec<&Slot> = self.slots(class).collect();
        match self.need(class) {
            Some(Need::Owned) => ClassRole::NeedsOwned,
            Some(Need::View) => ClassRole::KeepsView,
            _ if members.iter().any(|slot| self.is_stored(slot)) => ClassRole::Stored,
            _ if members.iter().any(|slot| slot.parameter().is_some()) => {
                ClassRole::Parameter { tied }
            }
            _ if members.iter().all(|slot| slot.is_local()) => ClassRole::Local,
            _ => ClassRole::Held,
        }
    }

    fn is_stored(&self, slot: &Slot) -> bool {
        slot.is_stored_place()
            || slot
                .parameter()
                .is_some_and(|binding| self.stored.contains(&binding))
    }

    fn slots(&self, class: NodeId) -> impl Iterator<Item = &'g Slot> + '_ {
        self.members
            .get(&class)
            .into_iter()
            .flatten()
            .map(|member| self.graph.slot(*member))
    }
}
