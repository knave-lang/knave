use super::ty::{Bound, GenericId, Type, TypeVar};
use crate::diagnostic::Diagnostic;
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
    OpenObligation {
        open_vars: Vec<TypeVar>,
        problem: Diagnostic,
    },
}
