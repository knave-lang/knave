use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, Suggestion};
use crate::source::Span;

pub struct LintProblems;

impl LintProblems {
    pub fn needless_type(function: &str, span: Span) -> Diagnostic {
        Self::needless_declaration(function, span, "inference gives this type")
            .with_suggestion("remove the declaration", Suggestion::new(span, ""))
    }

    pub fn needless_type_by_value(function: &str, parameters: &[String], span: Span) -> Diagnostic {
        let named = parameters
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(" and ");
        let (passes, small) = match parameters {
            [_] => ("passes", "it is"),
            _ => ("pass", "they are"),
        };
        let label = format!("inference gives this type, except that {named} {passes} by value");
        Self::needless_declaration(function, span, &label).with_suggestion(
            format!(
                "remove the declaration, and {named} {passes} by value, because {small} small enough to copy"
            ),
            Suggestion::new(span, ""),
        )
    }

    fn needless_declaration(function: &str, span: Span, label: &str) -> Diagnostic {
        Diagnostic::warning(
            DiagnosticCode::NEEDLESS_TYPE,
            format!("`{function}` does not need its `type` declaration"),
            Label::new(span, label),
        )
    }

    pub fn outside_rules(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::COMPILE_TIME_ERROR,
            format!("`{name}` is not a definition of the lint rules"),
            Label::new(span, "a lint rule reached it here"),
        )
    }

    pub fn missing_export(module: &str, name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::COMPILE_TIME_ERROR,
            format!("`{module}` exports no `{name}`"),
            Label::new(span, "the lint rules load it here"),
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
