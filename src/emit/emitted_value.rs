use crate::types::{TypedArgument, TypedExpr};

pub trait EmittedValue: Clone {
    fn expression(&self) -> &TypedExpr;

    fn with_expression(&self, expression: TypedExpr) -> Self;
}

impl EmittedValue for TypedExpr {
    fn expression(&self) -> &TypedExpr {
        self
    }

    fn with_expression(&self, expression: TypedExpr) -> Self {
        expression
    }
}

impl EmittedValue for TypedArgument {
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
