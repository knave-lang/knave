use super::value::Value;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub trait GlobalValues {
    fn global_value(&mut self, binding: BindingId, span: Span) -> Result<Value, Diagnostic>;
}
