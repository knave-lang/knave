use crate::source::Span;

#[derive(Clone, Copy)]
pub struct BuiltinSite {
    pub function: &'static str,
    pub span: Span,
}
