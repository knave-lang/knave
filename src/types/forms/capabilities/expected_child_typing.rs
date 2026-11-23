use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::expectation::Expectation;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait ExpectedChildTyping {
    fn type_child_against(
        &mut self,
        child: &Expr,
        expectation: Expectation,
    ) -> Result<TypedExpr, Diagnostic>;
}
