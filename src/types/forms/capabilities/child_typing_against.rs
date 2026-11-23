use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait ChildTypingAgainst {
    fn type_child_against(
        &mut self,
        child: &Expr,
        expected: &Type,
    ) -> Result<TypedExpr, Diagnostic>;
}
