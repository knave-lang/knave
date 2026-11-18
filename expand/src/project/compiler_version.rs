use crate::source::Span;

#[derive(Debug)]
pub struct CompilerVersion {
    pub text: String,
    pub span: Span,
}
