use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct KeptTemporary<'n> {
    pub argument: Span,
    pub callee: &'n str,
    pub parameter: &'n str,
    pub parameter_span: Span,
}

pub struct KeptTemporaryProblems;

impl KeptTemporaryProblems {
    pub fn kept_temporary(kept: &KeptTemporary<'_>) -> Diagnostic {
        let KeptTemporary {
            argument,
            callee,
            parameter,
            parameter_span,
        } = kept;
        Diagnostic::error(
            DiagnosticCode::KEPT_TEMPORARY_ARGUMENT,
            format!("the result of `{callee}` keeps a borrow of a temporary"),
            Label::new(
                *argument,
                "this temporary is dropped at the end of the statement",
            ),
        )
        .with_label(Label::new(
            *parameter_span,
            format!("the returned closure keeps this borrow of `{parameter}`"),
        ))
        .with_help(
            "bind the argument with `let` before the call, so it lives as long as the result",
        )
    }
}
