use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::ty::Type;

pub(in crate::types) trait GlobalReferences {
    fn is_unchecked(&self, binding: BindingId) -> bool;
    fn selfless_method_problem(
        &self,
        binding: BindingId,
        name: &str,
        span: Span,
    ) -> Option<Diagnostic>;
    fn global_type(&mut self, binding: BindingId) -> Type;
}
