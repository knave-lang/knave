use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct SecondRowProblems;

impl SecondRowProblems {
    pub fn second_row(span: Span, first: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::SECOND_MISMATCH_HELP,
            "a `mismatch-help` already states the help for these two types",
            Label::new(span, "second help for the same types"),
        )
        .with_label(Label::new(first, "first help"))
        .with_help("remove one of the two")
    }
}
