use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait OperandTyping {
    fn type_operand(&mut self, operand: &Expr) -> Result<TypedExpr, Diagnostic>;
}
