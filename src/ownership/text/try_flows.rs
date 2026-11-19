use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_value::TextValue;
use super::walk_context::WalkContext;
use crate::source::Span;

pub struct TryFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    span: Span,
}

impl<'c, 'w, 'p> TryFlows<'c, 'w, 'p> {
    const PAYLOAD: usize = 0;
    const ERROR: usize = 1;

    pub fn new(context: &'c mut WalkContext<'w, 'p>, span: Span) -> Self {
        Self { context, span }
    }

    pub fn propagate(&mut self, tried: &TextValue, returned: &SlotOwner) {
        for (path, node) in tried.nested() {
            if path.below(Self::ERROR).is_some() {
                let error = TextValue::from(Source::Slot(*node), self.span);
                self.context
                    .receive(&Slot::new(returned.clone(), path.clone()), &error);
            }
        }
    }

    pub fn payload(&self, tried: &TextValue) -> TextValue {
        tried
            .nested()
            .iter()
            .filter_map(|(path, node)| Some((path.below(Self::PAYLOAD)?, *node)))
            .fold(TextValue::default(), |value, (path, node)| {
                if path.is_top() {
                    value.merged(TextValue::from(Source::Slot(node), self.span))
                } else {
                    value.nesting(path, node)
                }
            })
    }
}
