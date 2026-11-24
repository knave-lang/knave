use crate::core::BindingId;
use crate::source::Span;
use crate::types::selfless_method::SelflessMethod;

pub(in crate::types) struct SelflessCall<'n> {
    pub binding: BindingId,
    pub method: SelflessMethod,
    pub name: &'n str,
    pub span: Span,
}
