use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct LimitProblems;

impl LimitProblems {
    pub fn too_many_steps(limit: u64, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MACRO_STEP_LIMIT,
            "macro code ran too many steps",
            Label::new(span, format!("this call passed {limit} macro steps")),
        )
        .with_note(format!(
            "the macro calls in one top-level form run at most {limit} steps: a step is one function call or one `recur`"
        ))
        .with_help("make the macro stop")
    }

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
