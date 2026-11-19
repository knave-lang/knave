use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::source::Span;

pub(super) struct ParameterCallRequest {
    pub callee: BindingId,
    pub position: usize,
    pub argument: Span,
}

impl ParameterCallRequest {
    pub(super) fn explains(&self, diagnostic: &Diagnostic) -> bool {
        diagnostic.code() == Some(&DiagnosticCode::MISMATCHED_TYPES)
            && diagnostic.primary().span() == self.argument
    }
}
