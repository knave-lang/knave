use super::ty::Type;
use crate::source::Span;

pub(super) struct OpenSetAt {
    pub(super) collection: Type,
    pub(super) key: Type,
    pub(super) item: Type,
    pub(super) span: Span,
}
