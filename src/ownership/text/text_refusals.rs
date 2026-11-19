use super::flow::Flow;
use super::need::Need;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::slot_classes::SlotClasses;
use super::source::Source;
use super::text_decision_table::TextDecisionTable;
use super::text_facts::TextFacts;
use super::transfer::Transfer;

pub struct TextRefusals<'r, 'g> {
    classes: &'r SlotClasses<'g>,
    facts: &'r TextFacts<'r, 'g>,
    table: &'r TextDecisionTable<'g>,
}

impl<'r, 'g> TextRefusals<'r, 'g> {
    pub fn new(
        classes: &'r SlotClasses<'g>,
        facts: &'r TextFacts<'r, 'g>,
        table: &'r TextDecisionTable<'g>,
    ) -> Self {
        Self {
            classes,
            facts,
            table,
        }
    }

    pub fn found(&self) -> Vec<&'g Flow> {
        self.classes
            .flows()
            .iter()
            .filter(|flow| self.refuses(flow))
            .collect()
    }

    fn refuses(&self, flow: &Flow) -> bool {
        let target = self.classes.class(flow.target);
        let arriving = self.facts.arriving(flow);
        let lends = flow.transfer == Transfer::Refer && self.classes.is_variable(target);
        self.need_refuses(flow, target)
            || !lends
                && !self.reported_upstream(flow)
                && self.table.of_class(target).refuses(arriving)
    }

    fn need_refuses(&self, flow: &Flow, target: NodeId) -> bool {
        let arriving = self.facts.arriving(flow);
        match self.classes.need(target) {
            Some(Need::Owned) => arriving.holds(ReachingText::STATIC),
            Some(Need::View) | None => false,
            Some(Need::OwnedOrMixed) => {
                let carries_text = arriving != ReachingText::NOTHING
                    || matches!(flow.source, Source::Slot(node) if self.classes.is_read_only_parameter(node));
                carries_text && !self.facts.at(target).holds(ReachingText::OWNED)
            }
        }
    }

    fn reported_upstream(&self, flow: &Flow) -> bool {
        match flow.source {
            Source::Slot(node) => {
                flow.transfer == Transfer::Move
                    && self
                        .table
                        .of_class(self.classes.class(node))
                        .takes_ownership()
            }
            Source::Text(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::text::flow_fixture::Flows;
    use crate::ownership::text::need::Need;
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::ownership::text::slot_owner::SlotOwner;
    use crate::ownership::text::transfer::Transfer;

    #[test]
    fn a_clone_of_a_parameter_with_no_owned_text_is_refused() {
        let flows = Flows::default()
            .flow((Flows::PARAMETER, Flows::variable()), Transfer::Refer, 7)
            .required(Flows::variable(), Need::OwnedOrMixed);
        assert_eq!(flows.refused_at(), vec![7]);
    }

    #[test]
    fn static_text_given_to_an_owned_argument_is_refused() {
        let owned = SlotOwner::Argument(Flows::span(80));
        let flows = Flows::default()
            .required(owned.clone(), Need::Owned)
            .text(ReachingText::STATIC, (owned.clone(), 4))
            .text(ReachingText::OWNED, (owned, 5));
        assert_eq!(flows.refused_at(), vec![4]);
    }

    #[test]
    fn an_owned_need_outranks_a_written_view() {
        let owned = SlotOwner::Argument(Flows::span(80));
        let flows = Flows::default()
            .required(owned.clone(), Need::Owned)
            .required(Flows::field(), Need::View)
            .united(owned.clone(), Flows::field())
            .text(ReachingText::STATIC, (owned, 4));
        assert_eq!(flows.refused_at(), vec![4]);
    }
}
