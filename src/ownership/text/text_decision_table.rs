use rustc_hash::FxHashMap;

use super::class_role::ClassRole;
use super::flow_graph::FlowGraph;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot::Slot;
use super::slot_classes::SlotClasses;
use super::text_facts::TextFacts;

pub struct TextDecisionTable<'g> {
    by_class: FxHashMap<NodeId, Representation>,
    by_slot: FxHashMap<&'g Slot, Representation>,
}

impl<'g> TextDecisionTable<'g> {
    pub fn decide(
        graph: &'g FlowGraph,
        classes: &SlotClasses<'g>,
        facts: &TextFacts<'_, 'g>,
    ) -> Self {
        let by_class: FxHashMap<NodeId, Representation> = classes
            .roles()
            .into_iter()
            .map(|(class, role)| (class, role.decide(facts.at(class), |tied| facts.at(tied))))
            .collect();
        let by_slot = graph
            .partition()
            .classes()
            .into_iter()
            .flat_map(|(class, members)| {
                let decided = by_class[&class];
                members
                    .into_iter()
                    .map(move |member| (graph.slot(member), decided))
            })
            .collect();
        Self { by_class, by_slot }
    }

    pub fn of_class(&self, class: NodeId) -> Representation {
        self.by_class[&class]
    }

    pub fn of_slot(&self, slot: &Slot) -> Representation {
        self.by_slot.get(slot).copied().unwrap_or_else(|| {
            ClassRole::unseen(slot).decide(ReachingText::NOTHING, |_| ReachingText::NOTHING)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::text::flow_fixture::Flows;
    use crate::ownership::text::need::Need;
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::ownership::text::representation::Representation;
    use crate::ownership::text::slot_owner::SlotOwner;
    use crate::ownership::text::transfer::Transfer;

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
        let owned = SlotOwner::Argument(Flows::span(80));
        let flows = Flows::default()
            .text(ReachingText::STATIC, (Flows::PARAMETER, 0))
            .flow((Flows::PARAMETER, owned.clone()), Transfer::Move, 1)
            .required(owned, Need::Owned);
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
    fn an_unseen_field_is_owned_and_an_unseen_local_a_view() {
        let flows = Flows::default();
        assert_eq!(flows.representation(Flows::field()), Representation::Owned);
        assert_eq!(flows.representation(Flows::LOCAL), Representation::View);
    }
}
