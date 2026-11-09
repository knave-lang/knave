use super::{
    TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf,
};
use crate::core::BindingId;

pub struct ReturnedValues<'e> {
    lambda: &'e TypedLambda,
}

impl<'e> ReturnedValues<'e> {
    pub fn of(lambda: &'e TypedLambda) -> Vec<&'e TypedExpr> {
        Self { lambda }.block(&lambda.body)
    }

    fn reached(&self, expr: &'e TypedExpr) -> Vec<&'e TypedExpr> {
        match &expr.kind {
            TypedExprKind::Control(control) => self.branches(control),
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self
                .bound_value(*binding)
                .map_or_else(|| vec![expr], |value| self.reached(value)),
            _ => vec![expr],
        }
    }

    fn branches(&self, control: &'e TypedControl) -> Vec<&'e TypedExpr> {
        match control {
            TypedControl::Block(block) => self.block(block),
            TypedControl::If(_, then, otherwise) => [then, otherwise]
                .into_iter()
                .flat_map(|branch| self.reached(branch))
                .collect(),
            TypedControl::Match(matched) => matched
                .arms
                .iter()
                .flat_map(|arm| self.block(&arm.body))
                .collect(),
        }
    }

    fn block(&self, block: &'e TypedBlock) -> Vec<&'e TypedExpr> {
        block
            .tail()
            .map_or_else(Vec::new, |tail| self.reached(tail))
    }

    fn bound_value(&self, binding: BindingId) -> Option<&'e TypedExpr> {
        self.lambda.body.items.iter().find_map(|item| match item {
            TypedBodyItem::Let(binder, value) if binder.binding == binding => Some(value),
            _ => None,
        })
    }
}
