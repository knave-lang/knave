use super::access::Access;
use super::loan_names::LoanNames;
use super::loan_place::LoanPlace;
use crate::source::Span;

pub struct PlaceAccess {
    place: LoanPlace,
    access: &'static dyn Access,
    span: Span,
}

impl PlaceAccess {
    pub fn new(place: LoanPlace, access: &'static dyn Access, span: Span) -> Self {
        Self {
            place,
            access,
            span,
        }
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        format!(
            "{} {} `{}`",
            names.position(self.span),
            self.access.verb(),
            self.place.written(names)
        )
    }
}
