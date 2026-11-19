use rustc_hash::FxHashMap;

use super::flow::Flow;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::slot_classes::SlotClasses;
use super::source::Source;

pub struct TextFacts<'c, 'g> {
    classes: &'c SlotClasses<'g>,
    reaching: FxHashMap<NodeId, ReachingText>,
}

impl<'c, 'g> TextFacts<'c, 'g> {
    pub fn spread(classes: &'c SlotClasses<'g>) -> Self {
        let mut facts = Self {
            classes,
            reaching: FxHashMap::default(),
        };
        while facts.spread_once() {}
        facts
    }

    pub fn at(&self, class: NodeId) -> ReachingText {
        self.reaching.get(&class).copied().unwrap_or_default()
    }

    pub fn arriving(&self, flow: &Flow) -> ReachingText {
        flow.transfer.carry(self.emitted(flow))
    }

    fn spread_once(&mut self) -> bool {
        let mut changed = false;
        for flow in self.classes.flows() {
            let target = self.classes.class(flow.target);
            let before = self.at(target);
            let after = before.with(self.arriving(flow));
            if after != before {
                self.reaching.insert(target, after);
                changed = true;
            }
        }
        changed
    }

    fn emitted(&self, flow: &Flow) -> ReachingText {
        match flow.source {
            Source::Text(text) => text,
            Source::Slot(node) if self.speaks_for_itself(node, flow.target) => {
                ReachingText::BORROWED
            }
            Source::Slot(node) => self.at(self.classes.class(node)),
        }
    }

    fn speaks_for_itself(&self, node: NodeId, target: NodeId) -> bool {
        self.classes.is_read_only_parameter(node)
            && !self.classes.is_variable(self.classes.class(target))
    }
}
