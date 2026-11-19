use super::flow_graph::FlowGraph;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use crate::types::Type;

pub struct SlotReads<'g> {
    graph: &'g mut FlowGraph,
}

impl<'g> SlotReads<'g> {
    pub fn new(graph: &'g mut FlowGraph) -> Self {
        Self { graph }
    }

    pub fn read(self, owner: &SlotOwner, read: TextRead<'_>) -> TextValue {
        let positions = TextPositions::of(read.ty);
        if !positions.iter().any(|position| position.path.is_top()) {
            return self.nested_onto(TextValue::default(), owner, read.ty);
        }
        let top = self.graph.node(Slot::from(owner.clone()));
        let value = TextValue::from(Source::Slot(top), read.span).transferred(read.transfer);
        self.nested_onto(value, owner, read.ty)
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
