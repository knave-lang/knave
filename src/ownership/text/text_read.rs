use super::transfer::Transfer;
use crate::source::Span;
use crate::types::Type;

pub struct TextRead<'t> {
    pub ty: &'t Type,
    pub span: Span,
    pub transfer: Transfer,
}
