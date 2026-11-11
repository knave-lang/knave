use std::rc::Rc;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, Suggestion};
use crate::source::Span;

pub struct WarningProblems;

impl WarningProblems {
    pub fn missing_lint(description: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::FAILURE_PART,
            "a `warn` description needs a `:code` part",
            Label::new(description, "this description names no lint"),
        )
        .with_help("name the lint, as in `(:code unmatched-link)`")
    }

    pub fn unknown_lint(name: &str, span: Span, similar: Option<Rc<str>>) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::UNKNOWN_FAILURE_CODE,
            format!("no lint is named `{name}`"),
            Label::new(span, "unknown lint"),
        );
        match similar {
            Some(similar) => diagnostic.with_suggestion(
                "a lint with a similar name exists",
                Suggestion::new(span, similar.as_ref()),
            ),
            None => diagnostic,
        }
    }
}
