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

#[cfg(test)]
mod tests {
    use super::TextSolver;
    use crate::core::BindingId;
    use crate::ownership::text::flow_graph::FlowGraph;
    use crate::ownership::text::node::{NodeId, Source};
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::ownership::text::representation::Representation;
    use crate::ownership::text::slot::{Slot, SlotOwner};
    use crate::ownership::text::text_value::TextValue;
    use crate::ownership::text::transfer::Transfer;
    use crate::source::{SourceId, Span};
    use crate::types::GenericId;

    #[derive(Default)]
    struct Flows {
        graph: FlowGraph,
    }

    impl Flows {
        const PARAMETER: SlotOwner = SlotOwner::Parameter(BindingId::from_index(1));
        const OTHER_PARAMETER: SlotOwner = SlotOwner::Parameter(BindingId::from_index(2));
        const LOCAL: SlotOwner = SlotOwner::Local(BindingId::from_index(3));
        const OTHER_LOCAL: SlotOwner = SlotOwner::Local(BindingId::from_index(4));

        fn span(at: usize) -> Span {
            Span::new(SourceId::new(0), at..at + 1)
        }

        fn field() -> SlotOwner {
            SlotOwner::Field(BindingId::from_index(5), "name".into())
        }

        fn variable() -> SlotOwner {
            SlotOwner::Variable(Self::span(90), GenericId::new(0))
        }

        fn node(&mut self, owner: &SlotOwner) -> NodeId {
            self.graph.node(Slot::top(owner.clone()))
        }

        fn text(mut self, kind: ReachingText, (into, at): (SlotOwner, usize)) -> Self {
            let value = TextValue::from(Source::Text(kind), Self::span(at));
            self.graph.receive(&Slot::top(into), &value);
            self
        }

        fn flow(
            mut self,
            (from, into): (SlotOwner, SlotOwner),
            transfer: Transfer,
            at: usize,
        ) -> Self {
            let source = self.node(&from);
            let value = TextValue::from(Source::Slot(source), Self::span(at)).transferred(transfer);
            self.graph.receive(&Slot::top(into), &value);
            self
        }

        fn united(mut self, first: SlotOwner, second: SlotOwner) -> Self {
            let (first, second) = (self.node(&first), self.node(&second));
            self.graph.unite(first, second);
            self
        }

        fn representation(mut self, owner: SlotOwner) -> Representation {
            let node = self.node(&owner);
            let solver = TextSolver::solve(&self.graph);
            solver.representation(solver.class(node))
        }

        fn refused_at(&self) -> Vec<usize> {
            TextSolver::solve(&self.graph)
                .refusals()
                .into_iter()
                .map(|flow| flow.span.range().start)
                .collect()
        }
    }

    #[test]
    fn a_parameter_moved_into_a_field_takes_its_callers_text() {
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, Flows::field()), Transfer::Move, 1);
        assert_eq!(
            flows.representation(Flows::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_lent_into_a_field_is_not_stored() {
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, Flows::field()), Transfer::Refer, 1);
        assert_eq!(flows.representation(Flows::PARAMETER), Representation::View);
    }

    #[test]
    fn a_parameter_stored_through_two_locals_is_stored() {
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, Flows::LOCAL), Transfer::Move, 1)
            .flow((Flows::LOCAL, Flows::OTHER_LOCAL), Transfer::Move, 2)
            .flow((Flows::OTHER_LOCAL, Flows::field()), Transfer::Move, 3);
        assert_eq!(
            flows.representation(Flows::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_passed_to_an_owned_argument_is_stored() {
        let owned = SlotOwner::OwnedArgument(Flows::span(80));
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, owned), Transfer::Move, 1);
        assert_eq!(
            flows.representation(Flows::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_passed_to_a_stored_parameter_is_stored() {
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow(
                (Flows::PARAMETER, Flows::OTHER_PARAMETER),
                Transfer::Move,
                1,
            )
            .flow((Flows::OTHER_PARAMETER, Flows::field()), Transfer::Move, 2);
        assert_eq!(
            flows.representation(Flows::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_read_only_parameter_passes_on_a_view_not_its_callers_text() {
        let flows = Flows::default()
            .text(ReachingText::OWNED, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, Flows::LOCAL), Transfer::Move, 1);
        assert_eq!(flows.representation(Flows::LOCAL), Representation::View);
    }

    #[test]
    fn a_tied_parameter_carries_its_callers_static_text() {
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, Flows::variable()), Transfer::Refer, 1)
            .text(ReachingText::OWNED, (Flows::variable(), 2));
        assert_eq!(
            flows.representation(Flows::PARAMETER),
            Representation::LentMixed
        );
    }

    #[test]
    fn a_clone_of_a_parameter_with_no_owned_text_is_refused() {
        let cloned = SlotOwner::ClonedArgument(Flows::span(80));
        let flows = Flows::default()
            .flow((Flows::PARAMETER, Flows::variable()), Transfer::Refer, 7)
            .united(Flows::variable(), cloned);
        assert_eq!(flows.refused_at(), vec![7]);
    }

    #[test]
    fn static_text_given_to_an_owned_argument_is_refused() {
        let owned = SlotOwner::OwnedArgument(Flows::span(80));
        let flows = Flows::default()
            .text(ReachingText::STATIC, (owned.clone(), 4))
            .text(ReachingText::OWNED, (owned, 5));
        assert_eq!(flows.refused_at(), vec![4]);
    }

    #[test]
    fn an_owned_need_outranks_a_written_view() {
        let owned = SlotOwner::OwnedArgument(Flows::span(80));
        let view = SlotOwner::WrittenView(BindingId::from_index(5), "name".into());
        let flows = Flows::default()
            .united(owned.clone(), view)
            .text(ReachingText::STATIC, (owned, 4));
        assert_eq!(flows.refused_at(), vec![4]);
    }
}
