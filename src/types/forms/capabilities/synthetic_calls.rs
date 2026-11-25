use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::ty::Type;
use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

pub(in crate::types) trait SyntheticCalls {
    fn type_synthetic_call(&mut self, call: SyntheticCall<'_>) -> Result<TypedExpr, Diagnostic>;
    fn pass_argument(&mut self, parameter: &Type, value: &TypedExpr) -> Result<Borrow, Diagnostic>;
    fn finish_call(
        &mut self,
        signature: SyntheticSignature<'_>,
        arguments: Vec<TypedArgument>,
        span: Span,
    ) -> TypedExpr;
}
