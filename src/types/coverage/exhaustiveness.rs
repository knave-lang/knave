use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::problems::TypeProblems;

pub(super) enum Exhaustiveness {
    Missing(Vec<String>),
    OnlyWithGuards,
    Complete,
}

impl Exhaustiveness {
    pub(super) fn report(&self, span: Span, diagnostics: &mut Diagnostics) {
        if let Exhaustiveness::Missing(cases) = self {
            diagnostics.push(TypeProblems::non_exhaustive_match(cases, span));
        }
    }

    pub(super) fn is_complete_without_guards(&self) -> bool {
        matches!(self, Exhaustiveness::Complete)
    }
}
