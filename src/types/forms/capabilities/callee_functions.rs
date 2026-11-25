use crate::diagnostic::Diagnostic;
use crate::types::function_shape::FunctionShape;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait CalleeFunctions {
    fn expect_function(
        &mut self,
        callee: &TypedExpr,
        argument_count: usize,
    ) -> Result<FunctionShape, Diagnostic>;
}
