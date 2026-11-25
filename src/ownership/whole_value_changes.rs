use crate::core::BindingId;
use crate::source::Span;
use crate::types::{Type, TypedArgument, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf};

pub struct WholeValueChanges;

impl WholeValueChanges {
    pub fn site(lambda: &TypedLambda, param: BindingId) -> Option<Span> {
        lambda
            .body
            .expressions()
            .find_map(|expr| Self::site_in(expr, param))
    }

    fn site_in(expr: &TypedExpr, param: BindingId) -> Option<Span> {
        let call = match &expr.kind {
            TypedExprKind::Call(callee, arguments) => Some((callee.as_ref(), arguments)),
            TypedExprKind::Assign(assignment) => {
                Some((&assignment.template, &assignment.arguments))
            }
            _ => None,
        };
        if let Some((callee, arguments)) = call
            && Self::replaces(callee, arguments, param)
        {
            return Some(callee.span);
        }
        expr.children()
            .into_iter()
            .find_map(|child| Self::site_in(child, param))
    }

    fn replaces(callee: &TypedExpr, arguments: &[TypedArgument], param: BindingId) -> bool {
        let Type::Function(_, params, _) = &callee.ty else {
            return false;
        };
        let [Type::Mut(_, referent), replacement] = params.as_slice() else {
            return false;
        };
        referent.as_ref() == replacement
            && matches!(
                arguments.first().map(|argument| &argument.value.kind),
                Some(TypedExprKind::Leaf(TypedLeaf::Local(binding))) if *binding == param
            )
    }
}
