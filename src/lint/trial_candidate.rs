use crate::core::{BindingId, ModulePath};
use crate::source::Span;

pub struct TrialCandidate<'d> {
    pub binding: BindingId,
    pub module: &'d ModulePath,
    pub span: Span,
    pub tried: bool,
}
