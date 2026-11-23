use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_value::TextValue;
use super::value_parts::ValueParts;
use super::walk_context::WalkContext;
use crate::source::Span;
use crate::types::Type;

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

    pub fn convert(&mut self, tried: &TextValue, (tried_type, returned_type): (&Type, &Type)) {
        let (Some(error), Some(Type::Named(target, _))) =
            (Self::error_of(tried_type), Self::error_of(returned_type))
        else {
            return;
        };
        let payloads = self.context.program().types().payloads();
        if let Some(case) = payloads.converted_case(*target, error) {
            let error = ValueParts::of(tried).part(Self::ERROR, self.span);
            self.context.receive(&Slot::from(case), &error);
        }
    }

    pub fn payload(&self, tried: &TextValue) -> TextValue {
        ValueParts::of(tried).part(Self::PAYLOAD, self.span)
    }

    fn error_of(result: &Type) -> Option<&Type> {
        match result {
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::error_of(inner),
            Type::Named(_, arguments) if arguments.len() == 2 => Some(&arguments[Self::ERROR]),
            _ => None,
        }
    }
}
