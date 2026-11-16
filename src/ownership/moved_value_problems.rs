use super::moved_values::MoveSite;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, Suggestion};
use crate::source::Span;

#[derive(Clone, Copy)]
pub struct MoveSpans {
    pub used: Span,
    pub moved: Span,
}

pub struct CopyBound<'n> {
    pub parameter: &'n str,
    pub copy_trait: &'n str,
}

struct MoveLabels {
    code: DiagnosticCode,
    used: &'static str,
    moved: &'static str,
}

impl MoveLabels {
    const SAME_PASS: Self = Self {
        code: DiagnosticCode::USE_AFTER_MOVE,
        used: "value used here after it moved",
        moved: "value moved here",
    };
    const EARLIER_PASS: Self = Self {
        code: DiagnosticCode::USE_AFTER_MOVE,
        used: "value used here after an earlier pass of the loop moved it",
        moved: "value moved here, in an earlier pass of the loop",
    };
    const MISSING_COPY_BOUND: Self = Self {
        code: DiagnosticCode::MISSING_COPY_BOUND,
        ..Self::SAME_PASS
    };
}

pub struct MovedValueProblems;

impl MovedValueProblems {
    pub fn use_after_move(name: &str, spans: MoveSpans) -> Diagnostic {
        Self::moved_value(name, spans, MoveLabels::SAME_PASS)
    }

    pub fn use_after_earlier_pass_move(name: &str, spans: MoveSpans) -> Diagnostic {
        Self::moved_value(name, spans, MoveLabels::EARLIER_PASS)
    }

    fn moved_value(name: &str, spans: MoveSpans, labels: MoveLabels) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            labels.code,
            format!("use of moved value `{name}`"),
            Label::new(spans.used, labels.used),
        );
        if spans.used == spans.moved {
            return diagnostic;
        }
        diagnostic.with_label(Label::new(spans.moved, labels.moved))
    }

    pub fn reuse_suggestions(diagnostic: Diagnostic, site: &MoveSite, name: &str) -> Diagnostic {
        let moved = site.span();
        let diagnostic = match site.borrowed_place() {
            Some(borrowed) => diagnostic.with_suggestion(
                "borrow the part if the first place only reads it",
                Suggestion::new(moved, borrowed.to_string()),
            ),
            None => diagnostic,
        };
        let cloned = format!("(clone {})", site.moved_place(name));
        diagnostic.with_suggestion(
            "clone the value if both places need it",
            Suggestion::new(moved, cloned),
        )
    }

    pub fn missing_copy_bound(name: &str, bound: &CopyBound, spans: MoveSpans) -> Diagnostic {
        let CopyBound {
            parameter,
            copy_trait,
        } = bound;
        Self::moved_value(name, spans, MoveLabels::MISSING_COPY_BOUND)
            .with_note(format!(
                "the signature writes `{parameter}` without `{copy_trait}`, and a written signature is exact"
            ))
            .with_help(format!(
                "add a `{copy_trait}` bound to the signature: `({parameter} {copy_trait})`"
            ))
    }
}
