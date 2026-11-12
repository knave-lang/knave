use super::temporary_scope::TemporaryScope;
use crate::types::TypedExpr;

pub enum Loan<'e> {
    Temporary(&'e TypedExpr),
    Operand(&'e TypedExpr, TemporaryScope),
}
