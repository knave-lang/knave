use rustc_hash::FxHashSet;

use super::flow::Flow;
use super::need::Need;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::refusal::{Refusal, RefusalCause};
use super::representation::Representation;
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

    pub fn found(&self) -> Vec<Refusal<'g>> {
        let candidates: Vec<(usize, Refusal<'g>)> = self
            .classes
            .flows()
            .iter()
            .enumerate()
            .filter_map(|(index, flow)| Some((index, self.refusal(flow)?)))
            .collect();
        let reported = self.reported_classes(&candidates);
        candidates
            .into_iter()
            .map(|(_, refusal)| refusal)
            .filter(|refusal| !self.repeats_a_report(refusal.flow, &reported))
            .flat_map(|refusal| self.at_written_sites(refusal))
            .collect()
    }

    fn at_written_sites(&self, refusal: Refusal<'g>) -> Vec<Refusal<'g>> {
        let Source::Slot(source) = refusal.flow.source else {
            return vec![refusal];
        };
        if !refusal.flow.origin.is_generated() {
            return vec![refusal];
        }
        let class = self.classes.class(source);
        let written: Vec<Refusal<'g>> = self
            .classes
            .flows()
            .iter()
            .filter(|flow| !flow.origin.is_generated() && self.classes.class(flow.target) == class)
            .filter(|flow| self.facts.arriving(flow).is_short_lived())
            .map(|flow| Refusal {
                flow,
                cause: refusal.cause,
            })
            .collect();
        if written.is_empty() {
            vec![refusal]
        } else {
            written
        }
    }

    fn reported_classes(&self, candidates: &[(usize, Refusal<'_>)]) -> FxHashSet<NodeId> {
        let refused: FxHashSet<usize> = candidates.iter().map(|(index, _)| *index).collect();
        let mut reported: FxHashSet<NodeId> = candidates
            .iter()
            .map(|(_, refusal)| self.classes.class(refusal.flow.target))
            .collect();
        while self.carry_reports(&mut reported) {}
        while self.drop_reports(&refused, &mut reported) {}
        reported
    }

    fn carry_reports(&self, reported: &mut FxHashSet<NodeId>) -> bool {
        let reached: Vec<NodeId> = self
            .classes
            .flows()
            .iter()
            .filter(|flow| self.comes_from_a_report(flow, reported))
            .filter(|flow| self.facts.arriving(flow).is_short_lived())
            .map(|flow| self.classes.class(flow.target))
            .collect();
        let before = reported.len();
        reported.extend(reached);
        reported.len() > before
    }

    fn drop_reports(&self, refused: &FxHashSet<usize>, reported: &mut FxHashSet<NodeId>) -> bool {
        let unexplained: Vec<NodeId> = self
            .classes
            .flows()
            .iter()
            .enumerate()
            .filter(|(index, flow)| {
                !refused.contains(index)
                    && !self.comes_from_a_report(flow, reported)
                    && self.facts.arriving(flow).is_short_lived()
            })
            .map(|(_, flow)| self.classes.class(flow.target))
            .collect();
        let before = reported.len();
        for class in unexplained {
            reported.remove(&class);
        }
        reported.len() < before
    }

    fn comes_from_a_report(&self, flow: &Flow, reported: &FxHashSet<NodeId>) -> bool {
        matches!(flow.source, Source::Slot(node) if reported.contains(&self.classes.class(node)))
    }

    fn repeats_a_report(&self, flow: &Flow, reported: &FxHashSet<NodeId>) -> bool {
        let lasting = self.facts.arriving(flow).without(ReachingText::SHORT_LIVED);
        self.comes_from_a_report(flow, reported) && !self.refuses(flow, lasting)
    }

    fn refusal(&self, flow: &'g Flow) -> Option<Refusal<'g>> {
        if flow.transfer == Transfer::Query {
            return self.query_refusal(flow);
        }
        let arriving = self.facts.arriving(flow);
        if !self.refuses(flow, arriving) {
            return None;
        }
        let cause = if self.refuses(flow, arriving.without(ReachingText::ASSUMED)) {
            RefusalCause::Rule
        } else {
            RefusalCause::StatedLimit
        };
        Some(Refusal { flow, cause })
    }

    fn query_refusal(&self, flow: &'g Flow) -> Option<Refusal<'g>> {
        let Source::Slot(asked) = flow.source else {
            return None;
        };
        let asked = self.facts.at(self.classes.class(asked));
        let key = self.table.of_class(self.classes.class(flow.target));
        Self::refuses_query(key, asked).then_some(Refusal {
            flow,
            cause: RefusalCause::Rule,
        })
    }

    fn refuses_query(key: Representation, asked: ReachingText) -> bool {
        match key {
            Representation::Owned => asked.holds(ReachingText::STATIC) || asked.is_short_lived(),
            Representation::Mixed => asked.is_short_lived(),
            _ => false,
        }
    }

    fn refuses(&self, flow: &Flow, arriving: ReachingText) -> bool {
        let target = self.classes.class(flow.target);
        let lends = flow.transfer.lends() && self.classes.is_variable(target);
        self.need_refuses(flow, (target, arriving))
            || !lends
                && !self.reported_upstream(flow)
                && self.table.of_class(target).refuses(arriving)
    }

    fn need_refuses(&self, flow: &Flow, (target, arriving): (NodeId, ReachingText)) -> bool {
        match self.classes.need(target) {
            Some(Need::Owned) => arriving.holds(ReachingText::STATIC),
            Some(Need::View | Need::Lent) | None => false,
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
    use super::TextRefusals;
    use crate::ownership::text::flow_fixture::FlowFixture;
    use crate::ownership::text::need::Need;
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::ownership::text::representation::Representation;
    use crate::ownership::text::slot_owner::SlotOwner;
    use crate::ownership::text::transfer::Transfer;

    #[test]
    fn a_store_in_generated_code_is_reported_where_written_code_hands_it_text() {
        let flows = FlowFixture::default()
            .text(ReachingText::BORROWED, (FlowFixture::PARAMETER, 4))
            .generated_flow(
                (FlowFixture::PARAMETER, FlowFixture::field()),
                Transfer::Move,
                9,
            );
        assert_eq!(flows.refused_at(), vec![4]);
    }

    #[test]
    fn a_store_in_generated_code_no_written_flow_reaches_stays_where_it_is() {
        let flows = FlowFixture::default()
            .generated_text(ReachingText::BORROWED, (FlowFixture::LOCAL, 2))
            .generated_flow(
                (FlowFixture::LOCAL, FlowFixture::field()),
                Transfer::Move,
                9,
            );
        assert_eq!(flows.refused_at(), vec![9]);
    }

    #[test]
    fn owned_keys_refuse_a_static_query_and_mixed_keys_take_it() {
        let (owned, mixed) = (Representation::Owned, Representation::Mixed);
        assert!(TextRefusals::refuses_query(owned, ReachingText::STATIC));
        assert!(!TextRefusals::refuses_query(mixed, ReachingText::STATIC));
        assert!(TextRefusals::refuses_query(mixed, ReachingText::BORROWED));
        assert!(!TextRefusals::refuses_query(
            Representation::View,
            ReachingText::BORROWED
        ));
    }

    #[test]
    fn a_clone_of_a_parameter_with_no_owned_text_is_refused() {
        let flows = FlowFixture::default()
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::variable()),
                Transfer::Refer,
                7,
            )
            .required(FlowFixture::variable(), Need::OwnedOrMixed);
        assert_eq!(flows.refused_at(), vec![7]);
    }

    #[test]
    fn a_clone_lent_owned_text_by_a_read_only_parameter_is_refused() {
        let flows = FlowFixture::default()
            .text(ReachingText::OWNED, (FlowFixture::PARAMETER, 3))
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::variable()),
                Transfer::Lend,
                7,
            )
            .required(FlowFixture::variable(), Need::OwnedOrMixed);
        assert_eq!(flows.refused_at(), vec![7]);
    }

    #[test]
    fn a_clone_lent_an_owned_local_is_not_refused() {
        let flows = FlowFixture::default()
            .text(ReachingText::OWNED, (FlowFixture::LOCAL, 3))
            .flow(
                (FlowFixture::LOCAL, FlowFixture::variable()),
                Transfer::Lend,
                7,
            )
            .required(FlowFixture::variable(), Need::OwnedOrMixed);
        assert!(flows.refused_at().is_empty());
    }

    #[test]
    fn a_clone_lent_a_lent_local_is_not_refused() {
        let flows = FlowFixture::default()
            .text(ReachingText::LENT, (FlowFixture::LOCAL, 3))
            .flow(
                (FlowFixture::LOCAL, FlowFixture::variable()),
                Transfer::Lend,
                7,
            )
            .required(FlowFixture::variable(), Need::OwnedOrMixed);
        assert!(flows.refused_at().is_empty());
    }

    #[test]
    fn a_refused_clone_argument_is_not_refused_again_at_its_result() {
        let flows = FlowFixture::default()
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::variable()),
                Transfer::Refer,
                7,
            )
            .required(FlowFixture::variable(), Need::OwnedOrMixed)
            .text(ReachingText::OWNED, (FlowFixture::PARAMETER, 3))
            .flow(
                (FlowFixture::variable(), FlowFixture::field()),
                Transfer::Move,
                9,
            );
        assert_eq!(flows.refused_at(), vec![7]);
    }

    #[test]
    fn a_class_with_an_unrefused_short_lived_arrival_is_refused_again() {
        let flows = FlowFixture::default()
            .text(ReachingText::OWNED, (FlowFixture::variable(), 5))
            .text(ReachingText::LENT, (FlowFixture::LOCAL, 3))
            .flow(
                (FlowFixture::LOCAL, FlowFixture::variable()),
                Transfer::Lend,
                4,
            )
            .text(ReachingText::BORROWED, (FlowFixture::variable(), 6))
            .flow(
                (FlowFixture::variable(), FlowFixture::field()),
                Transfer::Refer,
                9,
            );
        assert_eq!(flows.refused_at(), vec![6, 9]);
    }

    #[test]
    fn static_text_given_to_an_owned_argument_is_refused() {
        let owned = SlotOwner::Argument(FlowFixture::span(80));
        let flows = FlowFixture::default()
            .required(owned.clone(), Need::Owned)
            .text(ReachingText::STATIC, (owned.clone(), 4))
            .text(ReachingText::OWNED, (owned, 5));
        assert_eq!(flows.refused_at(), vec![4]);
    }

    #[test]
    fn a_short_lived_view_given_to_an_owned_argument_is_refused() {
        let owned = SlotOwner::Argument(FlowFixture::span(80));
        let flows = FlowFixture::default()
            .required(owned.clone(), Need::Owned)
            .text(ReachingText::BORROWED, (owned, 4));
        assert_eq!(flows.refused_at(), vec![4]);
    }

    #[test]
    fn an_assumed_view_given_to_an_owned_argument_is_refused_by_a_stated_limit() {
        let owned = SlotOwner::Argument(FlowFixture::span(80));
        let flows = FlowFixture::default()
            .required(owned.clone(), Need::Owned)
            .text(ReachingText::ASSUMED, (owned.clone(), 4))
            .text(ReachingText::BORROWED, (owned, 5));
        assert_eq!(flows.refused_at(), vec![4, 5]);
        assert_eq!(flows.refused_by_a_stated_limit_at(), vec![4]);
    }

    #[test]
    fn an_owned_need_outranks_a_written_view() {
        let owned = SlotOwner::Argument(FlowFixture::span(80));
        let flows = FlowFixture::default()
            .required(owned.clone(), Need::Owned)
            .required(FlowFixture::field(), Need::View)
            .united(owned.clone(), FlowFixture::field())
            .text(ReachingText::STATIC, (owned, 4));
        assert_eq!(flows.refused_at(), vec![4]);
    }
}
