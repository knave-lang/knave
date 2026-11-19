use super::ty::Type;
use crate::source::Span;

pub(super) struct ReopenedParameter {
    pub(super) variable: Type,
    pub(super) pinned: Type,
    pub(super) span: Span,
}
