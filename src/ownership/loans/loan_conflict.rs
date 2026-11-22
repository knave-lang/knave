use super::access::Access;
use super::conflict_parts::{ConflictParts, NamedHolder};
use super::labeled_spans::LabeledSpans;
use super::lending::Lending;
use super::loan_conflict_problems::LoanConflictProblems;
use super::loan_holder::LoanHolder;
use super::place_access::PlaceAccess;
use crate::core::BindingArena;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub struct LoanConflict<'b> {
    access: &'b PlaceAccess,
    lending: &'b Lending,
    conflicting: &'static dyn Access,
    holder: LoanHolder,
    later_use: Option<Span>,
}

impl<'b> LoanConflict<'b> {
    pub fn found(
        access: &'b PlaceAccess,
        lending: &'b Lending,
        holder: LoanHolder,
    ) -> Option<Self> {
        Some(Self {
            access,
            lending,
            conflicting: lending.excluding(access)?,
            holder,
            later_use: None,
        })
    }

    pub fn used_later_at(self, later_use: Option<Span>) -> Self {
        Self { later_use, ..self }
    }

    pub fn is_reported_in(&self, labeled: &LabeledSpans) -> bool {
        labeled.cover_any(&self.spans())
    }

    pub fn spans(&self) -> [Span; 2] {
        let mut spans = [self.access.span(), self.lending.lent().span()];
        spans.sort();
        spans
    }

    pub fn diagnostic(&self, bindings: &BindingArena) -> Diagnostic {
        let problems = LoanConflictProblems::new(self.parts(bindings));
        self.conflicting.conflict(&problems)
    }

    fn parts(&self, bindings: &BindingArena) -> ConflictParts {
        ConflictParts {
            place: self.access.named(bindings),
            lent_place: self.lending.lent().named(bindings),
            access_span: self.access.span(),
            lent_span: self.lending.lent().span(),
            holder: match self.holder {
                LoanHolder::Value(value) => {
                    NamedHolder::Value(bindings.get(value).name().to_string())
                }
                LoanHolder::Call => NamedHolder::Call,
                LoanHolder::Result => NamedHolder::Result,
            },
            later_use: self.later_use,
        }
    }
}
