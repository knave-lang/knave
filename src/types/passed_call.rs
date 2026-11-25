use super::typed::{TypedArgument, TypedExpr, TypedExprKind};

pub struct PassedCall<'e> {
    pub callee: &'e TypedExpr,
    pub arguments: &'e [TypedArgument],
}

impl<'e> PassedCall<'e> {
    pub fn of(expr: &'e TypedExpr) -> Option<Self> {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments) => Some(Self { callee, arguments }),
            TypedExprKind::Assign(assignment) => Some(Self {
                callee: &assignment.template,
                arguments: &assignment.arguments,
            }),
            _ => None,
        }
    }
}
