use super::local_borrows::ParameterOwner;
use super::problems::DroppedValue;
use crate::core::BindingId;
use crate::types::{
    TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda,
};

pub(super) struct OwnedValues<'e> {
    lambda: &'e TypedLambda,
    owner: ParameterOwner,
}

impl<'e> OwnedValues<'e> {
    pub fn new(lambda: &'e TypedLambda, owner: ParameterOwner) -> Self {
        Self { lambda, owner }
    }

    pub fn of(&self, binding: BindingId) -> Option<DroppedValue> {
        let parameter = self
            .lambda
            .params
            .iter()
            .find(|parameter| parameter.binding == binding);
        match parameter {
            Some(parameter) if parameter.ty.is_reference() => None,
            Some(parameter) => Some(self.owner.dropped_parameter(parameter.span)),
            None => Self::let_binder_in(&self.lambda.body, binding)
                .filter(|binder| !binder.ty.is_reference())
                .map(|binder| DroppedValue::Local(binder.span)),
        }
    }

    fn let_binder_in(block: &'e TypedBlock, binding: BindingId) -> Option<&'e TypedBinder> {
        block.items.iter().find_map(|item| match item {
            TypedBodyItem::Let(binder, _) if binder.binding == binding => Some(binder),
            TypedBodyItem::Let(_, value) | TypedBodyItem::Expr(value) => {
                Self::let_binder_inside(value, binding)
            }
        })
    }

    fn let_binder_inside(expr: &'e TypedExpr, binding: BindingId) -> Option<&'e TypedBinder> {
        match &expr.kind {
            TypedExprKind::Fn(_) => None,
            TypedExprKind::Control(TypedControl::Block(block)) => {
                Self::let_binder_in(block, binding)
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => matched
                .arms
                .iter()
                .find_map(|arm| Self::let_binder_in(&arm.body, binding)),
            _ => expr
                .children()
                .into_iter()
                .find_map(|child| Self::let_binder_inside(child, binding)),
        }
    }
}
