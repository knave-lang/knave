use crate::source::Span;

pub struct FallbackHelp {
    pub message: String,
    pub form: Span,
}

impl FallbackHelp {
    pub const WRITTEN_VALUE: &str = "{value}";
}
