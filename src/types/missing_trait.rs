use super::ty::{Bound, Type};
use crate::source::Span;

pub(super) struct MissingTrait<'b> {
    pub(super) bound: &'b Bound,
    pub(super) implementor: &'b Type,
    pub(super) span: Span,
    pub(super) shown_argument: Option<Type>,
}
