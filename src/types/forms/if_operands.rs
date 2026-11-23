use crate::core::Expr;

pub(in crate::types) struct IfOperands<'e> {
    pub(in crate::types) test: &'e Expr,
    pub(in crate::types) then: &'e Expr,
    pub(in crate::types) otherwise: &'e Expr,
}
