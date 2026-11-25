use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::interpret::InterpretProblems;
use crate::source::Span;

pub(super) struct KernelFormProblems;

impl KernelFormProblems {
    pub(super) fn program_only(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::PROGRAM_ONLY,
            format!("`{name}` belongs to the program, so macro code cannot run it"),
            Label::new(span, "macro code runs this"),
        )
        .with_help(InterpretProblems::RETURN_AS_CODE_HELP)
    }
}
