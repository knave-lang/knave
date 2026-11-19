use rustc_hash::{FxHashMap, FxHashSet};

use super::flow_graph::{Flow, FlowGraph};
use super::node::{NodeId, Source};
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot::{Need, Slot};
use super::stored_parameters::StoredParameters;
use super::transfer::Transfer;
use crate::core::BindingId;

pub struct TextSolver<'g> {
    graph: &'g FlowGraph,
    classes: FxHashMap<NodeId, Vec<NodeId>>,
    stored: FxHashSet<BindingId>,
    variables: FxHashSet<NodeId>,
    reaching: FxHashMap<NodeId, ReachingText>,
    representations: FxHashMap<NodeId, Representation>,
}

impl<'g> TextSolver<'g> {
    pub fn solve(graph: &'g FlowGraph) -> Self {
        let classes = graph.classes();
        let stored = StoredParameters::new(graph, &classes).find();
        let variables = classes
            .iter()
            .filter(|(_, members)| {
                members
                    .iter()
                    .any(|member| graph.slot(*member).is_variable())
            })
            .map(|(class, _)| *class)
            .collect();
        let mut solver = Self {
            graph,
            classes,
            stored,
            variables,
            reaching: FxHashMap::default(),
            representations: FxHashMap::default(),
        };
        while solver.spread() {}
        solver.representations = solver.decide();
        solver
    }

    pub fn representation(&self, class: NodeId) -> Representation {
        self.representations
            .get(&class)
            .copied()
            .unwrap_or(Representation::View)
    }

    pub fn refusals(&self) -> Vec<&'g Flow> {
        self.graph
            .flows()
            .iter()
            .filter(|flow| self.refuses(flow))
            .collect()
    }

    pub fn class(&self, node: NodeId) -> NodeId {
        self.graph.class(node)
    }

    fn spread(&mut self) -> bool {
        let mut changed = false;
        for flow in self.graph.flows() {
            let target = self.graph.class(flow.target);
            let before = self.reaching(target);
            let after = before.with(self.arriving(flow));
            if after != before {
                self.reaching.insert(target, after);
                changed = true;
            }
        }
        changed
    }

    fn arriving(&self, flow: &Flow) -> ReachingText {
        flow.transfer.carry(self.emitted(flow))
    }

    fn emitted(&self, flow: &Flow) -> ReachingText {
        match flow.source {
            Source::Text(text) => text,
            Source::Slot(node) if self.speaks_for_itself(node, flow.target) => {
                ReachingText::BORROWED
            }
            Source::Slot(node) => self.reaching(self.graph.class(node)),
        }
    }

    fn speaks_for_itself(&self, node: NodeId, target: NodeId) -> bool {
        self.is_read_only_parameter(node) && !self.variables.contains(&self.graph.class(target))
    }

    fn is_read_only_parameter(&self, node: NodeId) -> bool {
        self.graph
            .slot(node)
            .parameter()
            .is_some_and(|binding| !self.stored.contains(&binding))
    }

    fn decide(&self) -> FxHashMap<NodeId, Representation> {
        let tied = self.tied_variables();
        self.classes
            .keys()
            .map(|class| (*class, self.decided(*class, tied.get(class).copied())))
            .collect()
    }

    fn tied_variables(&self) -> FxHashMap<NodeId, NodeId> {
        self.graph
            .flows()
            .iter()
            .filter_map(|flow| match flow.source {
                Source::Slot(node) if self.is_read_only_parameter(node) => {
                    Some((self.graph.class(node), self.graph.class(flow.target)))
                }
                _ => None,
            })
            .filter(|(_, target)| self.variables.contains(target))
            .collect()
    }

    fn decided(&self, class: NodeId, tied: Option<NodeId>) -> Representation {
        let members: Vec<&Slot> = self.members(class).collect();
        let reaching = self.reaching(class);
        if members.iter().any(|slot| slot.need() == Some(Need::Owned)) {
            Representation::Owned
        } else if members.iter().any(|slot| slot.need() == Some(Need::View)) {
            Representation::View
        } else if members.iter().any(|slot| self.is_stored(slot)) {
            Representation::of_stored(reaching)
        } else if members.iter().any(|slot| slot.parameter().is_some()) {
            tied.map_or(Representation::View, |variable| {
                Representation::of_tied_parameter(self.reaching(variable))
            })
        } else if members.iter().all(|slot| slot.is_local()) {
            Representation::of_local(reaching)
        } else {
            Representation::of_held(reaching)
        }
    }

    fn is_stored(&self, slot: &Slot) -> bool {
        slot.is_stored_place()
            || slot
                .parameter()
                .is_some_and(|binding| self.stored.contains(&binding))
    }

    fn refuses(&self, flow: &Flow) -> bool {
        let target = self.graph.class(flow.target);
        let arriving = self.arriving(flow);
        let lends = flow.transfer == Transfer::Refer && self.variables.contains(&target);
        self.need_refuses(flow, target)
            || !lends
                && !self.reported_upstream(flow)
                && self.representation(target).refuses(arriving)
    }

    fn need_refuses(&self, flow: &Flow, target: NodeId) -> bool {
        let arriving = self.arriving(flow);
        match self.need(target) {
            Some(Need::Owned) => arriving.holds(ReachingText::STATIC),
            Some(Need::View) | None => false,
            Some(Need::OwnedOrMixed) => {
                let carries_text = arriving != ReachingText::NOTHING
                    || matches!(flow.source, Source::Slot(node) if self.is_read_only_parameter(node));
                carries_text && !self.reaching(target).holds(ReachingText::OWNED)
            }
        }
    }

    fn reported_upstream(&self, flow: &Flow) -> bool {
        match flow.source {
            Source::Slot(node) => {
                flow.transfer == Transfer::Move
                    && self
                        .representation(self.graph.class(node))
                        .takes_ownership()
            }
            Source::Text(_) => false,
        }
    }

    fn need(&self, class: NodeId) -> Option<Need> {
        self.members(class)
            .filter_map(Slot::need)
            .min_by_key(|need| *need != Need::Owned)
    }

    fn reaching(&self, class: NodeId) -> ReachingText {
        self.reaching.get(&class).copied().unwrap_or_default()
    }

    fn members(&self, class: NodeId) -> impl Iterator<Item = &'g Slot> + '_ {
        self.classes
            .get(&class)
            .into_iter()
            .flatten()
            .map(|member| self.graph.slot(*member))
    }
}
