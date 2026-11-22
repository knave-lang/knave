use super::parameter_owner::ParameterOwner;
use super::problems::DroppedValue;
use crate::core::BindingId;
use crate::types::{
    TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda,
    TypedMatch,
};

#[derive(Clone, Copy)]
pub(super) struct OwnedValues<'e> {
    lambda: &'e TypedLambda,
    owner: ParameterOwner,
}

impl<'e> OwnedValues<'e> {
    pub fn new(lambda: &'e TypedLambda, owner: ParameterOwner) -> Self {
        Self { lambda, owner }
    }

    pub fn of(&self, binding: BindingId) -> Option<DroppedValue> {
        match self
            .lambda
            .params
            .iter()
            .find(|parameter| parameter.binding == binding)
        {
            Some(parameter) if parameter.ty.is_reference() => None,
            Some(parameter) => Some(self.owner.dropped_parameter(parameter.span)),
            None => Self::owned_binder_in(&self.lambda.body, binding)
                .map(|binder| DroppedValue::Local(binder.span)),
        }
    }

    fn owned_binder_in(block: &'e TypedBlock, binding: BindingId) -> Option<&'e TypedBinder> {
        block.items.iter().find_map(|item| match item {
            TypedBodyItem::Let(binder, _) if binder.binding == binding => Some(binder),
            TypedBodyItem::Let(_, value) | TypedBodyItem::Expr(value) => {
                Self::owned_binder_inside(value, binding)
            }
        })
    }

    fn owned_binder_inside(expr: &'e TypedExpr, binding: BindingId) -> Option<&'e TypedBinder> {
        match &expr.kind {
            TypedExprKind::Fn(_) => None,
            TypedExprKind::Control(TypedControl::Block(block)) => {
                Self::owned_binder_in(block, binding)
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                Self::owned_arm_binder(matched, binding)
            }
            _ => expr
                .children()
                .into_iter()
                .find_map(|child| Self::owned_binder_inside(child, binding)),
        }
    }

    fn owned_arm_binder(matched: &'e TypedMatch, binding: BindingId) -> Option<&'e TypedBinder> {
        let owns_its_value = !matched.value.ty.is_reference();
        matched
            .arms
            .iter()
            .find_map(|arm| {
                let bound = arm
                    .pattern
                    .binders()
                    .into_iter()
                    .find(|binder| binder.binding == binding);
                match bound {
                    Some(binder) => owns_its_value.then_some(binder),
                    None => Self::owned_binder_in(&arm.body, binding),
                }
            })
            .or_else(|| Self::owned_binder_inside(&matched.value, binding))
    }
}
