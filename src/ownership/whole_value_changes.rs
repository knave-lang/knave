use crate::core::BindingId;
use crate::source::Span;
use crate::types::{TypedExpr, TypedExprKind, TypedLambda, WholeWrite};

pub struct WholeValueChanges;

impl WholeValueChanges {
    pub fn site(lambda: &TypedLambda, param: BindingId) -> Option<Span> {
        lambda
            .body
            .expressions()
            .find_map(|expr| Self::site_in(expr, param))
    }

    fn site_in(expr: &TypedExpr, param: BindingId) -> Option<Span> {
        if let TypedExprKind::Assign(assignment) = &expr.kind
            && WholeWrite::of(assignment).and_then(|write| write.local()) == Some(param)
        {
            return Some(expr.span);
        }
        expr.children()
            .into_iter()
            .find_map(|child| Self::site_in(child, param))
    }
}
