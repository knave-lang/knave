use rustc_hash::{FxHashMap, FxHashSet};

use super::class_role::ClassRole;
use super::flow::Flow;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot_classes::SlotClasses;
use super::source::Source;
use super::transfer::Transfer;

pub struct TextFacts<'c, 'g> {
    classes: &'c SlotClasses<'g>,
    views: FxHashSet<NodeId>,
    reaching: FxHashMap<NodeId, ReachingText>,
}

impl<'c, 'g> TextFacts<'c, 'g> {
    pub fn spread(classes: &'c SlotClasses<'g>) -> Self {
        let roles = classes.roles();
        let mut facts = Self::spread_handing_views(classes, FxHashSet::default());
        loop {
            let views = facts.view_classes(&roles);
            if views.len() == facts.views.len() {
                return facts;
            }
            facts = Self::spread_handing_views(classes, views);
        }
    }

    pub fn at(&self, class: NodeId) -> ReachingText {
        self.reaching.get(&class).copied().unwrap_or_default()
    }

    pub fn arriving(&self, flow: &Flow) -> ReachingText {
        self.transfer(flow).carry(self.emitted(flow))
    }

    fn spread_handing_views(classes: &'c SlotClasses<'g>, views: FxHashSet<NodeId>) -> Self {
        let mut facts = Self {
            classes,
            views,
            reaching: FxHashMap::default(),
        };
        while facts.spread_once() {}
        facts
    }

    fn view_classes(&self, roles: &[(NodeId, ClassRole)]) -> FxHashSet<NodeId> {
        roles
            .iter()
            .filter(|(class, role)| self.hands_on_a_view(*class, *role))
            .map(|(class, _)| *class)
            .chain(self.views.iter().copied())
            .collect()
    }

    fn hands_on_a_view(&self, class: NodeId, role: ClassRole) -> bool {
        let decided = role.decide(self.at(class), |tied| self.at(tied));
        role.hands_on_its_decision()
            && matches!(decided, Representation::View | Representation::StaticView)
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
            Source::Slot(node) => self.handed_on(self.classes.class(node)),
        }
    }

    fn handed_on(&self, class: NodeId) -> ReachingText {
        let text = self.at(class);
        if self.views.contains(&class) {
            Transfer::View.carry(text)
        } else {
            text
        }
    }

    fn transfer(&self, flow: &Flow) -> Transfer {
        match flow.source {
            Source::Slot(node) if self.classes.is_read_only_parameter(node) => {
                flow.transfer.without_referent()
            }
            _ => flow.transfer,
        }
    }

    fn speaks_for_itself(&self, node: NodeId, target: NodeId) -> bool {
        self.classes.is_read_only_parameter(node)
            && !self.classes.is_variable(self.classes.class(target))
    }
}
