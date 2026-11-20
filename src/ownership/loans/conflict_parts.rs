use super::loan_holder::LoanHolder;
use crate::source::Span;

pub struct ConflictParts {
    pub place: String,
    pub lent_place: String,
    pub access_span: Span,
    pub lent_span: Span,
    pub holder: LoanHolder,
    pub holder_name: Option<String>,
    pub later_use: Option<Span>,
}
