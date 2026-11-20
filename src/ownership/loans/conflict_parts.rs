use crate::source::Span;

pub enum NamedHolder {
    Value(String),
    Call,
    Result,
}

pub struct ConflictParts {
    pub place: String,
    pub lent_place: String,
    pub access_span: Span,
    pub lent_span: Span,
    pub holder: NamedHolder,
    pub later_use: Option<Span>,
}
