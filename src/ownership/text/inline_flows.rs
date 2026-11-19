use super::flow_graph::FlowGraph;
use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use crate::types::{GenericId, TypedExpr};

pub struct InlineFlows<'g> {
    graph: &'g mut FlowGraph,
    call: &'g TypedExpr,
}

impl<'g> InlineFlows<'g> {
    const SHARED_VARIABLE: usize = 0;

    pub fn new(graph: &'g mut FlowGraph, call: &'g TypedExpr) -> Self {
        Self { graph, call }
    }

    pub fn value(self, arguments: &[TextValue]) -> TextValue {
        let owner = SlotOwner::Variable(self.call.span, GenericId::new(Self::SHARED_VARIABLE));
        let variable = self.graph.node(Slot::from(owner.clone()));
        for argument in arguments {
            self.graph
                .receive(&Slot::from(owner.clone()), &Self::top_of(argument));
            for (_, nested) in argument.nested() {
                self.graph.unite(variable, *nested);
            }
        }
        self.result(variable)
    }

    fn result(&self, variable: NodeId) -> TextValue {
        TextPositions::of(&self.call.ty).into_iter().fold(
            TextValue::default(),
            |value, position| {
                if position.path.is_top() {
                    value.merged(TextValue::from(Source::Slot(variable), self.call.span))
                } else {
                    value.nesting(position.path, variable)
                }
            },
        )
    }

    fn top_of(argument: &TextValue) -> TextValue {
        argument
            .sources()
            .iter()
            .fold(TextValue::default(), |value, (source, transfer, span)| {
                value.merged(TextValue::from(*source, *span).transferred(*transfer))
            })
    }
}
