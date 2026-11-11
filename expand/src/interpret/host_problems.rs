use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct HostProblems;

impl HostProblems {
    pub fn missing_group_of_builtin(builtin: &str, span: Span) -> Diagnostic {
        Self::missing_group(
            format!("`{builtin}` is not available where this code runs"),
            span,
        )
        .with_help("this built-in works only inside a macro")
    }

    pub fn missing_global_values(span: Span) -> Diagnostic {
        Self::missing_group("macro code cannot read a global here".to_string(), span)
    }

    fn missing_group(title: String, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_HOST_GROUP,
            title,
            Label::new(span, "used here"),
        )
    }
}
