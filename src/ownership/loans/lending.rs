use super::loan_kind::LoanKind;
use super::loan_names::LoanNames;
use super::loan_place::LoanPlace;
use super::place_access::PlaceAccess;
use crate::source::Span;

pub struct Lending {
    place: LoanPlace,
    kind: &'static dyn LoanKind,
    span: Span,
}

impl Lending {
    pub fn new(place: LoanPlace, kind: &'static dyn LoanKind, span: Span) -> Self {
        Self { place, kind, span }
    }

    pub fn access(&self) -> PlaceAccess {
        PlaceAccess::new(self.place.clone(), self.kind.access(), self.span)
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        format!(
            "{}`{}` lent at {}",
            self.kind.written_prefix(),
            self.place.written(names),
            names.position(self.span)
        )
    }
}
