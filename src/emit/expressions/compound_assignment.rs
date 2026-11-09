use super::ExpressionEmitter;
use crate::types::{TypedArgument, TypedExpr, TypedExprKind};

impl ExpressionEmitter<'_> {
    const ASSIGNMENT_TEMPLATE: &'static str = "{0} = {1}";
    const DEREFERENCE: char = '*';

    pub(super) fn compound_assignment(
        &self,
        template: &str,
        arguments: &[TypedArgument],
    ) -> Option<(String, TypedArgument)> {
        let dereferences = template.strip_suffix(Self::ASSIGNMENT_TEMPLATE)?;
        if !dereferences
            .chars()
            .all(|character| character == Self::DEREFERENCE)
        {
            return None;
        }
        let [target, value] = arguments else {
            return None;
        };
        let TypedExprKind::Call(callee, operands) = &value.value.kind else {
            return None;
        };
        let [left, right] = operands.as_slice() else {
            return None;
        };
        let operator = self.compound_operator(callee)?;
        Self::same_place(&target.value, &left.value).then(|| {
            (
                format!("{dereferences}{{0}} {operator}= {{1}}"),
                right.clone(),
            )
        })
    }

    fn same_place(target: &TypedExpr, operand: &TypedExpr) -> bool {
        let operand = match &operand.kind {
            TypedExprKind::Deref(copied) => copied,
            _ => operand,
        };
        let (Some(target), Some(operand)) = (target.field_place(), operand.field_place()) else {
            return false;
        };
        target.root == operand.root
            && target
                .path
                .iter()
                .zip(&operand.path)
                .all(|(left, right)| left.name == right.name)
    }
}
