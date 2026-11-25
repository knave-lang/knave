use super::typed::{TypedArgument, TypedExpr, TypedExprKind};

pub struct PassedCallMut<'e> {
    pub callee: &'e mut TypedExpr,
    pub arguments: &'e mut [TypedArgument],
}

impl<'e> PassedCallMut<'e> {
    pub fn of(kind: &'e mut TypedExprKind) -> Option<Self> {
        match kind {
            TypedExprKind::Call(callee, arguments) => Some(Self { callee, arguments }),
            TypedExprKind::Assign(assignment) => Some(Self {
                callee: &mut assignment.template,
                arguments: &mut assignment.arguments,
            }),
            _ => None,
        }
    }
}
