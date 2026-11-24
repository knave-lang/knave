use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;
use crate::types::typed::{TypedArgument, TypedExpr};

pub(in crate::types) trait ArgumentTyping {
    fn type_arguments(
        &mut self,
        callee: Option<&TypedExpr>,
        parameters: &[Type],
        arguments: &[Expr],
    ) -> Result<Vec<TypedArgument>, Diagnostic>;
}
