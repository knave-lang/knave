use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct HostProblems;

impl HostProblems {
    pub(super) fn missing_group_of_builtin(builtin: &str, span: Span) -> Diagnostic {
        Self::missing_group(&format!("`{builtin}`"), span)
    }

    pub(super) fn missing_global_values(span: Span) -> Diagnostic {
        Self::missing_group("reading a global", span)
    }

    fn missing_group(what: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_HOST_GROUP,
            format!("{what} is not available where this code runs"),
            Label::new(span, "used here"),
        )
    }
}
