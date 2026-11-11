use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct HostProblems;

impl HostProblems {
    pub(super) fn missing_group(what: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_HOST_GROUP,
            format!("{what} is not available where this code runs"),
            Label::new(span, "used here"),
        )
    }
}
