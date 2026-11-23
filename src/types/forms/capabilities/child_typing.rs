use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait ChildTyping {
    fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic>;
}
