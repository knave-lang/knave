use super::flow_graph::FlowGraph;
use super::slot::Slot;
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

    pub fn read(self, base: &Slot, read: TextRead<'_>) -> TextValue {
        if !TextPositions::holds_top_text(read.ty) {
            return self.nested_onto(TextValue::default(), base, read.ty);
        }
        let top = self.graph.node(base.clone());
        let value = TextValue::from(Source::Slot(top), read.span).transferred(read.transfer);
        self.nested_onto(value, base, read.ty)
    }

    fn nested_onto(self, read: TextValue, base: &Slot, ty: &Type) -> TextValue {
        TextPositions::of(ty)
            .into_iter()
            .filter(|position| !position.path.is_top())
            .fold(read, |value, position| {
                let nested = self.graph.node(base.nested(&position.path));
                value.nesting(position.path, nested)
            })
    }
}
