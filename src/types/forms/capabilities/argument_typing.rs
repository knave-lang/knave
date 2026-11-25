use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;
use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

pub(in crate::types) trait ArgumentTyping {
    fn type_arguments(
        &mut self,
        callee: Option<&TypedExpr>,
        parameters: &[Type],
        arguments: &[Expr],
    ) -> Result<Vec<TypedArgument>, Diagnostic>;
    fn pass_first_argument(
        &mut self,
        callee: &TypedExpr,
        parameters: &[Type],
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic>;
}
