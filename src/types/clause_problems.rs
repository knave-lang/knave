use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct ClauseProblems;

impl ClauseProblems {
    pub(super) fn clause_drops_a_borrow(argument: usize, passed: &str, clause: Span) -> Diagnostic {
        Self::dropped_borrow(
            clause,
            format!(
                "this clause passes `{{{argument}}}` without the template's borrow, and here `{{{argument}}}` is `{passed}`, not a shared reference"
            ),
        )
    }

    pub(super) fn trait_clause_drops_a_borrow(argument: usize, clause: Span) -> Diagnostic {
        Self::dropped_borrow(
            clause,
            format!(
                "this clause names a trait, so it cannot pass `{{{argument}}}` without the template's borrow"
            ),
        )
    }

    fn dropped_borrow(clause: Span, reason: String) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CLAUSE_PASSES_ARGUMENTS_UNLIKE_TEMPLATE,
            "a `rust` clause must pass its arguments as its template does",
            Label::new(clause, reason),
        )
        .with_help("borrow the placeholder as the template does")
    }

    pub(super) fn covered_clause(clause: Span, earlier: Span) -> Diagnostic {
        Self::unreachable_clause(clause, earlier, "an earlier clause covers this target")
            .with_help("move the narrower clause first")
    }

    pub(super) fn repeated_clause(clause: Span, earlier: Span) -> Diagnostic {
        Self::unreachable_clause(clause, earlier, "an earlier clause has this target")
            .with_help("remove the repeated clause")
    }

    fn unreachable_clause(clause: Span, earlier: Span, reason: &str) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::COVERED_CLAUSE,
            "this clause can never be chosen",
            Label::new(clause, reason),
        )
        .with_label(Label::new(earlier, "this clause comes first"))
    }
}
