use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct LifetimeProblems;

impl LifetimeProblems {
    pub fn unnamed_result_borrow(result: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNNAMED_RESULT_BORROW,
            "this template's result borrows, and its parameters hold more than one reference",
            Label::new(result, "borrows from which parameter?"),
        )
        .with_help(
            "name the parameter it borrows from with a lifetime, as in \
             `(where ('a) (fn ((ref 'a str) (ref str)) (ref 'a str)))`",
        )
    }
}
