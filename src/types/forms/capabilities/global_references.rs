use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::forms::SelflessCall;
use crate::types::selfless_method::SelflessMethod;
use crate::types::ty::Type;

pub(in crate::types) trait GlobalReferences {
    fn is_unchecked(&self, binding: BindingId) -> bool;
    fn selfless_method(&self, binding: BindingId) -> Option<SelflessMethod>;
    fn selfless_method_problem(&self, call: SelflessCall<'_>) -> Diagnostic;
    fn global_type(&mut self, binding: BindingId, span: Span) -> Type;
}
