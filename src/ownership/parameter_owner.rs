use super::problems::DroppedValue;
use crate::source::Span;

#[derive(Clone, Copy)]
pub(super) enum ParameterOwner {
    Function,
    Closure,
}

impl ParameterOwner {
    pub(super) fn dropped_parameter(self, span: Span) -> DroppedValue {
        match self {
            Self::Function => DroppedValue::FunctionParameter(span),
            Self::Closure => DroppedValue::ClosureParameter(span),
        }
    }
}
