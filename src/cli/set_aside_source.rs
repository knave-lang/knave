use crate::source::Span;

#[derive(Clone)]
pub struct SetAsideSource {
    pub name: String,
    pub span: Span,
}
