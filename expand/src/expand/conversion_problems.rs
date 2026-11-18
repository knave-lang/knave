use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct ConversionProblems;

impl ConversionProblems {
    pub(super) fn in_clause_template(span: Span) -> Diagnostic {
        Self::misplaced(span, "a clause template cannot convert a placeholder")
            .with_help("write the conversion in the template after `rust`")
    }

    pub(super) fn in_type_template(position: usize, span: Span) -> Diagnostic {
        Self::misplaced(span, "a type template cannot convert a placeholder")
            .with_help(format!("write the placeholder as `{{{position}}}`"))
    }

    fn misplaced(span: Span, label: &str) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MALFORMED_TEMPLATE,
            "this Rust template has a malformed placeholder",
            Label::new(span, label),
        )
    }
}
