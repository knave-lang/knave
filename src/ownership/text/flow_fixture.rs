use super::call_results::CallResults;
use super::flow_graph::FlowGraph;
use super::flow_origin::FlowOrigin;
use super::need::Need;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::refusal::RefusalCause;
use super::refusal_sites::RefusalSites;
use super::representation::Representation;
use super::slot::Slot;
use super::slot_classes::SlotClasses;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_decision_table::TextDecisionTable;
use super::text_facts::TextFacts;
use super::text_refusals::TextRefusals;
use super::text_value::TextValue;
use super::transfer::Transfer;
use crate::core::BindingId;
use crate::source::{SourceId, Span};
use crate::types::GenericId;

pub struct FlowFixture {
    graph: FlowGraph,
    origin: FlowOrigin,
}

impl Default for FlowFixture {
    fn default() -> Self {
        Self {
            graph: FlowGraph::default(),
            origin: FlowOrigin::Written,
        }
    }
}

impl FlowFixture {
    pub const PARAMETER: SlotOwner = SlotOwner::Parameter(BindingId::from_index(1));
    pub const OTHER_PARAMETER: SlotOwner = SlotOwner::Parameter(BindingId::from_index(2));
    pub const LOCAL: SlotOwner = SlotOwner::Local(BindingId::from_index(3));
    pub const OTHER_LOCAL: SlotOwner = SlotOwner::Local(BindingId::from_index(4));

    pub fn span(at: usize) -> Span {
        Span::new(SourceId::new(0), at..at + 1)
    }

    pub fn field() -> SlotOwner {
        SlotOwner::Field(BindingId::from_index(5), "name".into())
    }

    pub fn variable() -> SlotOwner {
        SlotOwner::Variable(Self::span(90), GenericId::new(0))
    }

    pub fn text(mut self, kind: ReachingText, (into, at): (SlotOwner, usize)) -> Self {
        let value = TextValue::from(Source::Text(kind), Self::span(at));
        self.graph.receive(&Slot::from(into), &value, self.origin);
        self
    }

    pub fn flow(
        mut self,
        (from, into): (SlotOwner, SlotOwner),
        transfer: Transfer,
        at: usize,
    ) -> Self {
        let source = self.node(&from);
        let value = TextValue::from(Source::Slot(source), Self::span(at)).transferred(transfer);
        self.graph.receive(&Slot::from(into), &value, self.origin);
        self
    }

    pub fn in_generated_code(mut self) -> Self {
        self.origin = FlowOrigin::Generated;
        self
    }

    pub fn united(mut self, first: SlotOwner, second: SlotOwner) -> Self {
        let (first, second) = (self.node(&first), self.node(&second));
        self.graph.unite(first, second);
        self
    }

    pub fn required(mut self, owner: SlotOwner, need: Need) -> Self {
        self.graph.require(Slot::from(owner), need);
        self
    }

    pub fn representation(&self, owner: SlotOwner) -> Representation {
        let classes = SlotClasses::of(&self.graph);
        let calls = CallResults::of(&self.graph, &classes);
        let facts = TextFacts::spread(&classes, &calls);
        TextDecisionTable::decide(&self.graph, &classes, &facts).of_slot(&Slot::from(owner))
    }

    pub fn refused_at(&self) -> Vec<usize> {
        self.refusals_where(|_| true)
    }

    pub fn refused_by_a_stated_limit_at(&self) -> Vec<usize> {
        self.refusals_where(|cause| cause == RefusalCause::StatedLimit)
    }

    fn refusals_where(&self, wanted: impl Fn(RefusalCause) -> bool) -> Vec<usize> {
        let classes = SlotClasses::of(&self.graph);
        let calls = CallResults::of(&self.graph, &classes);
        let facts = TextFacts::spread(&classes, &calls);
        let table = TextDecisionTable::decide(&self.graph, &classes, &facts);
        let found = TextRefusals::new(&classes, &facts, &table).found();
        RefusalSites::new(&classes, &facts)
            .placed(found)
            .into_iter()
            .filter(|refusal| wanted(refusal.cause))
            .map(|refusal| refusal.flow.span.range().start)
            .collect()
    }

    fn node(&mut self, owner: &SlotOwner) -> NodeId {
        self.graph.node(Slot::from(owner.clone()))
    }
}
