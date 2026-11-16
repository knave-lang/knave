use std::rc::Rc;

use super::quoted_list::QuotedList;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, SimilarName, Suggestion};
use crate::source::Span;

pub(super) struct LintNameProblems;

impl LintNameProblems {
    const ALWAYS_PRINTED: [DiagnosticCode; 5] = [
        DiagnosticCode::SECOND_TRAIT_PATH,
        DiagnosticCode::KERNEL_TYPE_HIDDEN,
        DiagnosticCode::INPUT_SET_ASIDE,
        DiagnosticCode::LIBRARY_HIDES_LOCAL_MODULE,
        DiagnosticCode::TEST_MODULE_HIDES_SOURCE_MODULE,
    ];

    const BARE_NAME: &str = "write the lint's name bare: the name that `knave doc <code>` shows for an `L08xx` code, with dashes in place of its spaces";

    pub(super) fn unknown_lint(name: &str, span: Span, lints: &[Rc<str>]) -> Diagnostic {
        let diagnostic = Self::unknown_name(name, span);
        match SimilarName::closest(name, lints) {
            Some(closest) => diagnostic.with_suggestion(
                "a lint with a similar name exists",
                Suggestion::new(span, &*closest),
            ),
            None => diagnostic
                .with_note(Self::always_printed())
                .with_help(format!("the lints are {}", QuotedList::of(lints))),
        }
    }

    pub(super) fn code_as_name(code: &str, span: Span, name: &str) -> Diagnostic {
        Self::unknown_name(code, span)
            .with_suggestion(Self::code_help(code), Suggestion::new(span, name))
    }

    pub(super) fn quoted_code(code: &str, span: Span, name: &str) -> Diagnostic {
        Self::not_a_symbol(span).with_suggestion(Self::code_help(code), Suggestion::new(span, name))
    }

    pub(super) fn name_not_a_symbol(span: Span, lint: Option<&str>) -> Diagnostic {
        let diagnostic = Self::not_a_symbol(span);
        match lint {
            Some(name) => diagnostic.with_suggestion(
                "write the lint's name without quotes",
                Suggestion::new(span, name),
            ),
            None => diagnostic.with_help(Self::BARE_NAME),
        }
    }

    pub(super) fn listed_twice(name: &str, span: Span, first: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LINT_LISTED_TWICE,
            format!("lint `{name}` is listed twice"),
            Label::new(span, "listed again here"),
        )
        .with_label(Label::new(first, "first listed here"))
        .with_help("keep one entry for each lint")
    }

    fn not_a_symbol(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LINT_NAME_NOT_A_SYMBOL,
            "a lint's name is a symbol",
            Label::new(span, "not a symbol"),
        )
    }

    fn code_help(code: &str) -> String {
        format!("`{code}` is the lint's code; write its name")
    }

    fn unknown_name(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNKNOWN_LINT,
            format!("unknown lint `{name}`"),
            Label::new(span, "not the name of a lint"),
        )
    }

    fn always_printed() -> String {
        let codes: Vec<&str> = Self::ALWAYS_PRINTED
            .iter()
            .map(DiagnosticCode::as_str)
            .collect();
        format!("{} always print and have no level", QuotedList::of(&codes))
    }
}
