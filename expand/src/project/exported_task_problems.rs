use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct ExportedTaskProblems;

impl ExportedTaskProblems {
    pub fn unexpected_name(span: Span, section: &str) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNEXPECTED_PROJECT_FORM,
            "expected a task's name",
            Label::new(span, "this is not a name"),
        )
        .with_help(format!("list each task by its name, as `({section} lint)`"))
    }
}
