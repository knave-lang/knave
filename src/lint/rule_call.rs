use crate::core::BindingId;
use crate::interpret::Value;
use crate::reader::Form;
use crate::source::Span;

pub struct RuleCall {
    pub entry: BindingId,
    pub rule: Form,
    pub table: Value,
    pub span: Span,
    pub step_limit: u64,
}
