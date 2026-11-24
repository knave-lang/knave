use super::promoted_constants::PromotedConstants;
use crate::types::{TypedExpr, TypedExprKind, TypedProgram};

pub struct StatementTemporary<'p> {
    constants: PromotedConstants<'p>,
}

impl<'p> StatementTemporary<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            constants: PromotedConstants::new(program),
        }
    }

    pub fn of<'e>(&self, argument: &'e TypedExpr) -> Option<&'e TypedExpr> {
        let value = match &argument.kind {
            TypedExprKind::Borrow(_, value) => value,
            _ => argument,
        };
        let owns_its_value = !value.ty.is_reference() && !value.is_place();
        (owns_its_value && !self.constants.is_promoted(value)).then_some(value)
    }
}
