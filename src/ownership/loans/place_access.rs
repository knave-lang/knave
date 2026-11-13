use super::access::Access;
use super::loan_id::LoanId;
use super::loan_names::LoanNames;
use super::loan_place::LoanPlace;
use crate::source::Span;

pub struct PlaceAccess {
    place: LoanPlace,
    access: &'static dyn Access,
    span: Span,
    lent: Option<LoanId>,
}

impl PlaceAccess {
    pub fn new(place: LoanPlace, access: &'static dyn Access, span: Span) -> Self {
        Self {
            place,
            access,
            span,
            lent: None,
        }
    }

    pub fn lending(self, loan: LoanId) -> Self {
        Self {
            lent: Some(loan),
            ..self
        }
    }

    pub fn is_lent_as(&self, loan: LoanId) -> bool {
        self.lent == Some(loan)
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
