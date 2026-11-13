use super::loan_place::LoanPlace;
use crate::core::BindingArena;
use crate::source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Change,
    Move,
    Borrow,
}

pub struct PlaceAccess {
    place: LoanPlace,
    kind: AccessKind,
    span: Span,
}

impl PlaceAccess {
    pub fn new(place: LoanPlace, kind: AccessKind, span: Span) -> Self {
        Self { place, kind, span }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn written(&self, bindings: &BindingArena) -> String {
        let verb = match self.kind {
            AccessKind::Change => "change",
            AccessKind::Move => "move",
            AccessKind::Borrow => "borrow",
        };
        format!("{verb} `{}`", self.place.written(bindings))
    }
}
