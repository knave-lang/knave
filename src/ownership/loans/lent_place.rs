use super::loan_place::LoanPlace;
use crate::core::{BindingArena, BindingId};
use crate::source::Span;

#[derive(Clone, Copy)]
pub struct LentPlace<'l> {
    place: &'l LoanPlace,
    span: Span,
}

impl<'l> LentPlace<'l> {
    pub fn new(place: &'l LoanPlace, span: Span) -> Self {
        Self { place, span }
    }

    pub fn named(&self, bindings: &BindingArena) -> String {
        self.place.named(bindings)
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn root(&self) -> BindingId {
        self.place.root()
    }
}
