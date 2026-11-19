use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
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

    pub fn value(self, arguments: &[TextValue]) -> TextValue {
        let owner = SlotOwner::Variable(self.call.span, GenericId::new(Self::SHARED_VARIABLE));
        let variable = self.context.node(Slot::from(owner.clone()));
        for argument in arguments {
            self.context
                .receive(&Slot::from(owner.clone()), &Self::top_of(argument));
            for (_, nested) in argument.nested() {
                self.context.unite(variable, *nested);
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
