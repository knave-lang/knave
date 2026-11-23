use super::call_target::CallTarget;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::walk_context::WalkContext;
use crate::types::TypedExpr;

pub struct CallResultReads<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    call: &'c TypedExpr,
}

impl<'c, 'w, 'p> CallResultReads<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, call: &'c TypedExpr) -> Self {
        Self { context, call }
    }

    pub fn read(
        self,
        read: TextValue,
        (target, values): (&dyn CallTarget<'_>, &[TextValue]),
    ) -> TextValue {
        if read.sources().is_empty() {
            return read;
        }
        let per_call = Slot::from(SlotOwner::CallResult(self.call.span));
        self.context.receive(&per_call, &Self::top(&read));
        for (index, value) in values.iter().enumerate() {
            let parameter = target.parameter(index).and_then(|slot| slot.parameter());
            if let Some(parameter) = parameter {
                let through = Self::top(value).transferred(Transfer::Through(parameter));
                self.context.receive(&per_call, &through);
            }
        }
        let node = self.context.node(per_call);
        read.nested().iter().fold(
            TextValue::from(super::source::Source::Slot(node), self.call.span),
            |value, (path, nested)| value.nesting(path.clone(), *nested),
        )
    }

    fn top(value: &TextValue) -> TextValue {
        value
            .sources()
            .iter()
            .fold(TextValue::default(), |top, (source, transfer, span)| {
                top.merged(TextValue::from(*source, *span).transferred(*transfer))
            })
    }
}
