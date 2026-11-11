use crate::types::{TypedArgument, TypedExpr};

pub trait ExpressionHolder: Clone {
    fn expression(&self) -> &TypedExpr;

    fn with_expression(&self, expression: TypedExpr) -> Self;
}

impl ExpressionHolder for TypedExpr {
    fn expression(&self) -> &TypedExpr {
        self
    }

    fn with_expression(&self, expression: TypedExpr) -> Self {
        expression
    }
}

impl ExpressionHolder for TypedArgument {
    fn expression(&self) -> &TypedExpr {
        &self.value
    }

    fn with_expression(&self, expression: TypedExpr) -> Self {
        Self {
            value: expression,
            borrow: self.borrow,
        }
    }
}
