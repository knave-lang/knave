use crate::diagnostic::{Counted, Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct RustTypeClauseProblems;

impl RustTypeClauseProblems {
    pub fn placeholder_without_parameter(
        index: usize,
        parameters: usize,
        span: Span,
    ) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::PLACEHOLDER_WITHOUT_PARAMETER,
            format!("`{{{index}}}` names no type parameter"),
            Label::new(
                span,
                format!(
                    "this type has {}",
                    Counted::noun(parameters, "type parameter")
                ),
            ),
        )
        .with_help("a clause template names the type's one parameter as `{0}`")
    }
}
