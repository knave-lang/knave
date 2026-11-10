use crate::source::Span;

pub(super) struct UnreachableArm {
    pub pattern: Span,
    pub covered_by: Option<Span>,
}
