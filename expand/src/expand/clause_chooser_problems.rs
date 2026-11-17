use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct ClauseChooserProblems;

impl ClauseChooserProblems {
    pub(super) fn missing(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::RUST_CLAUSES_NEED_A_PARAMETER,
            "a `rust` with clauses takes a type parameter",
            Label::new(span, "this type binds no type parameter"),
        )
        .with_help(
            "bind a parameter for the clauses to choose by, as in `(where ((s source)) (fn ((ref s)) string))`",
        )
    }

    pub(super) fn fixed_by_no_argument(chooser: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::RUST_CLAUSES_NEED_A_PARAMETER,
            "a `rust` with clauses chooses by its first type parameter, which no argument fixes",
            Label::new(chooser, "no argument's type names this parameter"),
        )
        .with_help("list first a parameter that an argument's type names")
    }
}
