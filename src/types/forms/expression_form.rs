use crate::diagnostic::Diagnostic;
use crate::types::expectation::Expectation;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait ExpressionForm<C> {
    fn type_against(
        &self,
        context: &mut C,
        expectation: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic>;
}
