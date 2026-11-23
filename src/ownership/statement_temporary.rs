use super::promoted_constants::PromotedConstants;
use crate::types::{TypedExpr, TypedExprKind};

pub struct StatementTemporary;

impl StatementTemporary {
    pub fn of<'e>(argument: &'e TypedExpr, constants: PromotedConstants) -> Option<&'e TypedExpr> {
        let value = match &argument.kind {
            TypedExprKind::Borrow(_, value) => value,
            _ => argument,
        };
        let owns_its_value = !value.ty.is_reference() && !value.is_place();
        (owns_its_value && !constants.is_promoted(value)).then_some(value)
    }
}
