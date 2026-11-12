use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct LintProblems;

impl LintProblems {
    pub fn outside_rules(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::COMPILE_TIME_ERROR,
            format!("`{name}` is not a definition of the lint rules"),
            Label::new(span, "a lint rule reached it here"),
        )
    }

    pub fn malformed_rule_names(entry: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::COMPILE_TIME_ERROR,
            format!("`{entry}` of the lint rules is not a list of rule names"),
            Label::new(span, "the lint rules were read for this definition"),
        )
    }

    pub fn rule_skipped(rules: &[String], definition: &str, span: Span) -> Diagnostic {
        let named: Vec<String> = rules.iter().map(|rule| format!("`{rule}`")).collect();
        let verb = if rules.len() == 1 { "was" } else { "were" };
        Diagnostic::warning(
            DiagnosticCode::RULE_SKIPPED,
            format!("{} {verb} not checked in `{definition}`", named.join(", ")),
            Label::new(
                span,
                "this definition is too large for the rule's step budget",
            ),
        )
        .with_help("split the definition into smaller ones")
    }
}
