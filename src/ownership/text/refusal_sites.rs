use super::flow::Flow;
use super::node_id::NodeId;
use super::refusal::Refusal;
use super::slot_classes::SlotClasses;
use super::source::Source;
use super::text_facts::TextFacts;

pub struct RefusalSites<'r, 'g> {
    classes: &'r SlotClasses<'g>,
    facts: &'r TextFacts<'r, 'g>,
}

impl<'r, 'g> RefusalSites<'r, 'g> {
    pub fn new(classes: &'r SlotClasses<'g>, facts: &'r TextFacts<'r, 'g>) -> Self {
        Self { classes, facts }
    }

    pub fn placed(&self, refusals: Vec<Refusal<'g>>) -> Vec<Refusal<'g>> {
        refusals
            .into_iter()
            .flat_map(|refusal| self.relocate(refusal))
            .collect()
    }

    fn relocate(&self, refusal: Refusal<'g>) -> Vec<Refusal<'g>> {
        let Source::Slot(source) = refusal.flow.source else {
            return vec![refusal];
        };
        if !refusal.flow.origin.is_generated() {
            return vec![refusal];
        }
        let written: Vec<Refusal<'g>> = self
            .written_feeds(self.classes.class(source))
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

    fn written_feeds(&self, class: NodeId) -> impl Iterator<Item = &'g Flow> + '_ {
        self.classes
            .flows()
            .iter()
            .filter(move |flow| {
                !flow.origin.is_generated() && self.classes.class(flow.target) == class
            })
            .filter(|flow| self.facts.brings_short_lived_text(flow))
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::text::flow_fixture::FlowFixture;
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::ownership::text::transfer::Transfer;

    #[test]
    fn a_store_in_generated_code_is_reported_where_written_code_hands_it_text() {
        let flows = FlowFixture::default()
            .text(ReachingText::BORROWED, (FlowFixture::PARAMETER, 4))
            .in_generated_code()
            .flow(
                (FlowFixture::PARAMETER, FlowFixture::field()),
                Transfer::Move,
                9,
            );
        assert_eq!(flows.refused_at(), vec![4]);
    }

    #[test]
    fn a_store_in_generated_code_no_written_flow_reaches_stays_where_it_is() {
        let flows = FlowFixture::default()
            .in_generated_code()
            .text(ReachingText::BORROWED, (FlowFixture::LOCAL, 2))
            .flow(
                (FlowFixture::LOCAL, FlowFixture::field()),
                Transfer::Move,
                9,
            );
        assert_eq!(flows.refused_at(), vec![9]);
    }
}
