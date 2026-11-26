use super::ty::{Bound, GenericId, Type};
use crate::source::Span;

pub(super) enum SolverEvent {
    MissingTrait {
        bound: Bound,
        implementor: Type,
        span: Span,
        shown_argument: Option<Type>,
    },
    StructuralUse {
        bound: Bound,
        implementor: Type,
        span: Span,
    },
    UnprovenBound {
        generic: GenericId,
        bound: Bound,
        span: Span,
    },
}
