use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct LimitProblems;

impl LimitProblems {
    pub fn value_nests_too_deep(limit: usize, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::EXPANSION_NESTS_TOO_DEEP,
            "macro code built a list nested too deeply",
            Label::new(span, format!("this call builds a list past {limit} levels")),
        )
        .with_note(format!(
            "lists that macro code builds nest at most {limit} deep"
        ))
        .with_help("make the macro stop nesting, or build a flatter list")
    }
}
