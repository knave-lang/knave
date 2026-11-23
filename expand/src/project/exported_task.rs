use crate::source::Span;

#[derive(Debug)]
pub struct ExportedTask {
    pub name: String,
    pub span: Span,
}
