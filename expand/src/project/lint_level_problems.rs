use std::rc::Rc;

use super::quoted_list::QuotedList;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, SimilarName, Suggestion};
use crate::source::Span;

pub(super) struct LintLevelProblems;

impl LintLevelProblems {
    pub(super) fn unknown_level(word: &str, span: Span, levels: &[Rc<str>]) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::UNKNOWN_LINT_LEVEL,
            format!("unknown lint level `{word}`"),
            Label::new(span, "not a level"),
        );
        match SimilarName::closest(word, levels) {
            Some(closest) => diagnostic.with_suggestion(
                "a level with a similar name exists",
                Suggestion::new(span, &*closest),
            ),
            None => diagnostic.with_help(format!("the levels are {}", QuotedList::of(levels))),
        }
    }

    pub(super) fn level_not_a_symbol(
        span: Span,
        written: Option<&str>,
        levels: &[Rc<str>],
    ) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::LINT_LEVEL_NOT_A_SYMBOL,
            "a lint's level is a symbol",
            Label::new(span, "not a symbol"),
        );
        match written {
            Some(level) => diagnostic.with_suggestion(
                "write the level without quotes",
                Suggestion::new(span, level),
            ),
            None => diagnostic.with_help(format!("the levels are {}", QuotedList::of(levels))),
        }
    }

    pub(super) fn two_levels(span: Span, first: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::DUPLICATE_PROJECT_ENTRY,
            "a lint entry gives two levels",
            Label::new(span, "a second level"),
        )
        .with_label(Label::new(first, "the first level"))
        .with_help("keep one level for each lint")
    }

    pub(super) fn without_level(span: Span, levels: &[Rc<str>]) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LINT_WITHOUT_LEVEL,
            "a lint entry gives no level",
            Label::new(span, "expected a lint's name, then its level"),
        )
        .with_help(format!(
            "give the lint one of the levels {}",
            QuotedList::of(levels)
        ))
    }

    pub(super) fn threshold(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LINT_THRESHOLD,
            "no rule takes a threshold yet",
            Label::new(span, "a threshold"),
        )
        .with_help("remove the number, and give the lint a level")
    }
}
