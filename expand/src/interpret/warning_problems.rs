use std::rc::Rc;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, RuleName, Suggestion};
use crate::source::Span;

pub struct WarningProblems;

impl WarningProblems {
    pub fn warned(rule: RuleName, failure: &Diagnostic) -> Diagnostic {
        let code = failure.code().expect("a `warn` description names its code");
        let warning = Diagnostic::warning(code.clone(), failure.title(), failure.primary().clone())
            .with_rule(rule);
        let labeled = failure
            .labels()
            .skip(1)
            .fold(warning, |warning, label| warning.with_label(label.clone()));
        let noted = failure
            .notes()
            .iter()
            .fold(labeled, |warning, note| warning.with_note(note.clone()));
        failure
            .helps()
            .iter()
            .fold(noted, |warning, help| warning.with_help(help.clone()))
    }

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

#[cfg(test)]
mod tests {
    use super::WarningProblems;
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, RuleName, Severity};
    use crate::source::{SourceId, Span};

    struct WarnedFixture;

    impl WarnedFixture {
        const RULE: &str = "unmatched-link";
        const HELP: &str = "move the van";
        const EARLIER: &str = "parked before";

        fn failure() -> Diagnostic {
            let span = Span::new(SourceId::new(0), 0..4);
            Diagnostic::error(
                DiagnosticCode::UNMATCHED_LINK,
                "the van is parked",
                Label::new(span, "here"),
            )
            .with_label(Label::new(span, Self::EARLIER))
            .with_help(Self::HELP)
        }

        fn warning() -> Diagnostic {
            WarningProblems::warned(RuleName::new(Self::RULE), &Self::failure())
        }
    }

    #[test]
    fn a_warning_carries_its_lint_rule() {
        let rule = WarnedFixture::warning().rule().cloned();
        assert_eq!(rule, Some(RuleName::new(WarnedFixture::RULE)));
    }

    #[test]
    fn a_warning_stays_a_warning() {
        assert_eq!(WarnedFixture::warning().severity(), Severity::Warning);
    }

    #[test]
    fn a_warning_keeps_the_help_of_its_description() {
        let warning = WarnedFixture::warning();
        assert_eq!(warning.helps()[0].message(), WarnedFixture::HELP);
    }

    #[test]
    fn a_warning_keeps_the_labels_of_its_description() {
        let warning = WarnedFixture::warning();
        let labels: Vec<&str> = warning.labels().map(|label| label.message()).collect();
        assert_eq!(labels, ["here", WarnedFixture::EARLIER]);
    }
}
