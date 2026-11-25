use crate::diagnostic::Diagnostic;
use crate::types::finished_call::FinishedCall;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait SyntheticCalls {
    fn type_synthetic_call(&mut self, call: SyntheticCall<'_>) -> Result<TypedExpr, Diagnostic>;
    fn finish_call(&mut self, call: FinishedCall<'_>) -> TypedExpr;
}
