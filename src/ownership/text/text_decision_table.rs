use rustc_hash::FxHashMap;

use super::class_role::ClassRole;
use super::flow_graph::FlowGraph;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot::Slot;
use super::slot_classes::SlotClasses;
use super::source::Source;
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
        let roles = classes.roles();
        let mut by_class: FxHashMap<NodeId, Representation> = roles
            .iter()
            .map(|(class, role)| (*class, role.decide(facts.at(*class), |tied| facts.at(tied))))
            .collect();
        let elements: Vec<NodeId> = roles
            .iter()
            .filter(|(_, role)| *role == ClassRole::ParameterElement)
            .map(|(class, _)| *class)
            .collect();
        for element in elements {
            if let Some(agreed) = Self::callers_of(element, classes, &by_class) {
                by_class.insert(element, agreed);
            }
        }
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

    fn callers_of(
        element: NodeId,
        classes: &SlotClasses<'_>,
        decided: &FxHashMap<NodeId, Representation>,
    ) -> Option<Representation> {
        let callers = classes.flows().iter().filter_map(|flow| match flow.source {
            Source::Slot(node)
                if classes.class(flow.target) == element && classes.class(node) != element =>
            {
                decided.get(&classes.class(node)).copied()
            }
            _ => None,
        });
        Self::agreed(callers)
    }

    fn agreed(callers: impl Iterator<Item = Representation>) -> Option<Representation> {
        callers
            .map(|caller| match caller {
                Representation::StaticView => Representation::View,
                other => other,
            })
            .reduce(|agreed, caller| {
                if agreed == caller {
                    agreed
                } else {
                    Representation::Generic
                }
            })
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
    use super::TextDecisionTable;
    use crate::core::BindingId;
    use crate::ownership::text::flow_fixture::FlowFixture;
    use crate::ownership::text::need::Need;
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::ownership::text::representation::Representation;
    use crate::ownership::text::slot_owner::SlotOwner;
    use crate::ownership::text::transfer::Transfer;

    #[test]
    fn callers_that_agree_decide_a_parameter_element() {
        let owned = [Representation::Owned, Representation::Owned];
        assert_eq!(
            TextDecisionTable::agreed(owned.into_iter()),
            Some(Representation::Owned)
        );
        let views = [Representation::StaticView, Representation::View];
        assert_eq!(
            TextDecisionTable::agreed(views.into_iter()),
            Some(Representation::View)
        );
    }

    #[test]
    fn callers_that_disagree_make_a_parameter_element_generic() {
        let callers = [Representation::Owned, Representation::View];
        assert_eq!(
            TextDecisionTable::agreed(callers.into_iter()),
            Some(Representation::Generic)
        );
        assert_eq!(TextDecisionTable::agreed(std::iter::empty()), None);
    }

    #[test]
    fn a_parameter_moved_into_a_field_takes_its_callers_text() {
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::field()),
                Transfer::Move,
                1,
            );
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_lent_into_a_field_is_not_stored() {
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::field()),
                Transfer::Refer,
                1,
            );
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::View
        );
    }

    #[test]
    fn a_parameter_stored_through_two_locals_is_stored() {
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::LOCAL),
                Transfer::Move,
                1,
            )
            .flow(
                (FlowFixture::LOCAL, FlowFixture::OTHER_LOCAL),
                Transfer::Move,
                2,
            )
            .flow(
                (FlowFixture::OTHER_LOCAL, FlowFixture::field()),
                Transfer::Move,
                3,
            );
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_passed_to_an_owned_argument_is_stored() {
        let owned = SlotOwner::Argument(FlowFixture::span(80));
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .flow((FlowFixture::PARAMETER, owned.clone()), Transfer::Move, 1)
            .required(owned, Need::Owned);
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_passed_to_a_stored_parameter_is_stored() {
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::OTHER_PARAMETER),
                Transfer::Move,
                1,
            )
            .flow(
                (FlowFixture::OTHER_PARAMETER, FlowFixture::field()),
                Transfer::Move,
                2,
            );
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_parameter_a_write_changes_takes_its_callers_text() {
        let written = SlotOwner::Written(FlowFixture::span(60));
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .united(FlowFixture::PARAMETER, written);
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::StaticView
        );
    }

    #[test]
    fn a_read_only_parameter_passes_on_a_view_not_its_callers_text() {
        let flows = FlowFixture::default()
            .text(ReachingText::OWNED, (FlowFixture::PARAMETER, 0))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::LOCAL),
                Transfer::Move,
                1,
            );
        assert_eq!(
            flows.representation(FlowFixture::LOCAL),
            Representation::View
        );
    }

    #[test]
    fn a_tied_parameter_carries_its_callers_static_text() {
        let flows = FlowFixture::default()
            .text(ReachingText::STATIC, (FlowFixture::PARAMETER, 0))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::variable()),
                Transfer::Refer,
                1,
            )
            .text(ReachingText::OWNED, (FlowFixture::variable(), 2));
        assert_eq!(
            flows.representation(FlowFixture::PARAMETER),
            Representation::LentMixed
        );
    }

    #[test]
    fn a_result_decided_a_view_hands_its_caller_a_view() {
        let result = SlotOwner::Result(BindingId::from_index(9));
        let flows = FlowFixture::default()
            .text(ReachingText::LENT, (result.clone(), 0))
            .flow((result, FlowFixture::LOCAL), Transfer::Move, 1);
        assert_eq!(
            flows.representation(FlowFixture::LOCAL),
            Representation::View
        );
    }

    #[test]
    fn a_reference_a_template_builds_hands_its_caller_a_lent_value() {
        let built = SlotOwner::Value(FlowFixture::span(70));
        let flows = FlowFixture::default()
            .text(ReachingText::LENT, (built.clone(), 0))
            .flow((built, FlowFixture::LOCAL), Transfer::Move, 1);
        assert_eq!(
            flows.representation(FlowFixture::LOCAL),
            Representation::Lent
        );
    }

    #[test]
    fn an_unseen_field_is_owned_and_an_unseen_local_a_view() {
        let flows = FlowFixture::default();
        assert_eq!(
            flows.representation(FlowFixture::field()),
            Representation::Owned
        );
        assert_eq!(
            flows.representation(FlowFixture::LOCAL),
            Representation::View
        );
    }
}
