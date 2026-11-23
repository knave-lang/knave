use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::value_parts::ValueParts;
use super::walk_context::WalkContext;
use crate::types::{GenericId, TypedExpr};

pub struct InlineFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    call: &'c TypedExpr,
}

impl<'c, 'w, 'p> InlineFlows<'c, 'w, 'p> {
    const SHARED_VARIABLE: usize = 0;

    pub fn new(context: &'c mut WalkContext<'w, 'p>, call: &'c TypedExpr) -> Self {
        Self { context, call }
    }

    pub fn value(mut self, arguments: &[TextValue], queries: &[usize]) -> TextValue {
        let owner = SlotOwner::Variable(self.call.span, GenericId::new(Self::SHARED_VARIABLE));
        let variable = self.context.node(Slot::from(owner.clone()));
        let shared = arguments
            .iter()
            .enumerate()
            .filter(|(index, _)| !queries.contains(index))
            .map(|(_, argument)| argument);
        for argument in shared {
            self.context
                .receive(&Slot::from(owner.clone()), &ValueParts::of(argument).top());
            for (_, nested) in argument.nested() {
                self.context.unite(variable, *nested);
            }
        }
        self.result(variable)
    }

    fn result(&mut self, variable: NodeId) -> TextValue {
        let mut value = TextValue::default();
        for position in TextPositions::of(&self.call.ty) {
            let transfer = if position.is_written_reference() {
                Transfer::Refer
            } else {
                Transfer::Move
            };
            let read =
                TextValue::from(Source::Slot(variable), self.call.span).transferred(transfer);
            if position.path.is_top() {
                value = value.merged(read);
            } else {
                let slot = Slot::new(SlotOwner::Value(self.call.span), position.path.clone());
                self.context.receive(&slot, &read);
                value = value.nesting(position.path, self.context.node(slot));
            }
        }
        value
    }
}
