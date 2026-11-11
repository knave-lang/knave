use crate::reader::Symbol;
use crate::source::Span;

pub(super) struct NotedTerm {
    pub term: Symbol,
    pub span: Span,
}
