mod code;
mod code_stage;
mod collection;
mod counted;
mod distinct;
mod explanation;
mod label;
mod quoted_form;
mod render;
mod rule_name;
mod severity;
mod similar_name;
mod source_window;

pub use code::DiagnosticCode;
pub use code_stage::CodeStage;
pub use collection::Diagnostics;
pub use counted::Counted;
pub use distinct::DistinctDiagnostics;
pub use explanation::Explanation;
pub use label::{Help, Label, Suggestion};
pub use render::DiagnosticRenderer;
pub use rule_name::RuleName;
pub use severity::Severity;
pub use similar_name::SimilarName;

use std::rc::Rc;

use crate::core::Namespace;
use crate::source::{SourceId, Span};

#[derive(Clone, Debug)]
pub struct Diagnostic {
    report: Box<Report>,
}

#[derive(Clone, Debug)]
struct Report {
    severity: Severity,
    code: Option<DiagnosticCode>,
    title: String,
    primary: Label,
    secondary: Vec<Label>,
    notes: Vec<String>,
    helps: Vec<Help>,
    missing_name: Option<MissingName>,
    rule: Option<RuleName>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingName {
    pub name: Rc<str>,
    pub namespace: Namespace,
}

impl Diagnostic {
    pub fn error(code: DiagnosticCode, title: impl Into<String>, primary: Label) -> Self {
        Self::new(Severity::Error, Some(code), title, primary)
    }

    pub fn warning(code: DiagnosticCode, title: impl Into<String>, primary: Label) -> Self {
        Self::new(Severity::Warning, Some(code), title, primary)
    }

    pub fn note(title: impl Into<String>, primary: Label) -> Self {
        Self::new(Severity::Note, None, title, primary)
    }

    fn new(
        severity: Severity,
        code: Option<DiagnosticCode>,
        title: impl Into<String>,
        primary: Label,
    ) -> Self {
        Self {
            report: Box::new(Report {
                severity,
                code,
                title: title.into(),
                primary,
                secondary: Vec::new(),
                notes: Vec::new(),
                helps: Vec::new(),
                missing_name: None,
                rule: None,
            }),
        }
    }

    pub fn is_error(&self) -> bool {
        self.report.severity == Severity::Error
    }

    pub fn severity(&self) -> Severity {
        self.report.severity
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.report.secondary.push(label);
        self
    }

    pub fn without_labels_in(mut self, source: SourceId) -> Self {
        self.report
            .secondary
            .retain(|label| label.span().source() != source);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.report.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<Help>) -> Self {
        self.report.helps.push(help.into());
        self
    }

    pub fn with_quoting_help(mut self, help: impl Into<String>, form: Span) -> Self {
        self.report.helps.push(Help::quoting(help.into(), form));
        self
    }

    pub fn with_suggestion(mut self, help: impl Into<String>, suggestion: Suggestion) -> Self {
        self.report
            .helps
            .push(Help::new(help.into()).with_suggestion(suggestion));
        self
    }

    pub fn with_missing_name(mut self, missing: MissingName) -> Self {
        self.report.missing_name = Some(missing);
        self
    }

    pub fn missing_name(&self) -> Option<&MissingName> {
        self.report.missing_name.as_ref()
    }

    pub fn into_warning(mut self, rule: RuleName) -> Self {
        self.report.severity = Severity::Warning;
        self.report.rule = Some(rule);
        self
    }

    pub fn rule(&self) -> Option<&RuleName> {
        self.report.rule.as_ref()
    }

    pub fn code(&self) -> Option<&DiagnosticCode> {
        self.report.code.as_ref()
    }

    pub fn title(&self) -> &str {
        &self.report.title
    }

    pub fn labels(&self) -> impl Iterator<Item = &Label> {
        std::iter::once(&self.report.primary).chain(&self.report.secondary)
    }

    pub fn primary(&self) -> &Label {
        &self.report.primary
    }

    pub fn notes(&self) -> &[String] {
        &self.report.notes
    }

    pub fn helps(&self) -> &[Help] {
        &self.report.helps
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCode, Label, RuleName, Severity};
    use crate::source::{SourceId, Span};

    struct WarningFixture;

    impl WarningFixture {
        const RULE: &str = "unmatched-link";
        const HELP: &str = "move the van";
        const EARLIER: &str = "parked before";

        fn warning() -> Diagnostic {
            let span = Span::new(SourceId::new(0), 0..4);
            Diagnostic::error(
                DiagnosticCode::UNMATCHED_LINK,
                "the van is parked",
                Label::new(span, "here"),
            )
            .with_label(Label::new(span, Self::EARLIER))
            .with_help(Self::HELP)
            .into_warning(RuleName::new(Self::RULE))
        }
    }

    #[test]
    fn a_warning_carries_its_lint_rule() {
        let rule = WarningFixture::warning().rule().cloned();
        assert_eq!(rule, Some(RuleName::new(WarningFixture::RULE)));
    }

    #[test]
    fn a_warning_has_the_warning_severity() {
        assert_eq!(WarningFixture::warning().severity(), Severity::Warning);
    }

    #[test]
    fn a_warning_keeps_the_labels_and_help_it_was_built_with() {
        let warning = WarningFixture::warning();
        let labels: Vec<&str> = warning.labels().map(Label::message).collect();
        assert_eq!(labels, ["here", WarningFixture::EARLIER]);
        assert_eq!(warning.helps()[0].message(), WarningFixture::HELP);
    }
}
