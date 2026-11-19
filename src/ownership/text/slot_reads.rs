use super::flow_graph::FlowGraph;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use super::transfer::Transfer;
use crate::source::Span;
use crate::types::Type;

pub struct SlotReads<'g> {
    graph: &'g mut FlowGraph,
}

impl<'g> SlotReads<'g> {
    pub fn new(graph: &'g mut FlowGraph) -> Self {
        Self { graph }
    }

    pub fn read(
        self,
        owner: &SlotOwner,
        (ty, span): (&Type, Span),
        transfer: Transfer,
    ) -> TextValue {
        let positions = TextPositions::of(ty);
        if !positions.iter().any(|position| position.path.is_top()) {
            return self.nested_onto(TextValue::default(), owner, ty);
        }
        let top = self.graph.node(Slot::from(owner.clone()));
        let read = TextValue::from(Source::Slot(top), span).transferred(transfer);
        self.nested_onto(read, owner, ty)
    }

    pub fn nested(self, owner: &SlotOwner, ty: &Type) -> TextValue {
        self.nested_onto(TextValue::default(), owner, ty)
    }

    fn nested_onto(self, read: TextValue, owner: &SlotOwner, ty: &Type) -> TextValue {
        TextPositions::of(ty)
            .into_iter()
            .filter(|position| !position.path.is_top())
            .fold(read, |value, position| {
                let nested = self
                    .graph
                    .node(Slot::new(owner.clone(), position.path.clone()));
                value.nesting(position.path, nested)
            })
    }
}
