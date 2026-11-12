use crate::core::{BindingId, PatternArguments};
use crate::source::Span;

pub(super) struct ShortPattern<'a> {
    pub(super) name: &'a str,
    pub(super) candidates: &'a [BindingId],
    pub(super) written: &'a PatternArguments,
    pub(super) span: Span,
}
