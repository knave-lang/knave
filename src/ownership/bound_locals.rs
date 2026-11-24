use rustc_hash::FxHashMap;

use crate::core::BindingId;
use crate::types::{
    TypedArgument, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda,
    TypedLeaf, TypedMatch,
};

#[derive(Clone, Copy)]
pub enum BoundLocal<'e> {
    Value(&'e TypedExpr),
    Within(&'e TypedExpr),
}

pub struct BoundLocals<'e> {
    locals: FxHashMap<BindingId, BoundLocal<'e>>,
}

impl<'e> BoundLocals<'e> {
    pub fn in_argument(argument: &'e TypedExpr) -> Self {
        let mut bound = Self {
            locals: FxHashMap::default(),
        };
        bound.collect(argument);
        bound
    }

    pub fn of(&self, value: &TypedExpr) -> Option<BoundLocal<'e>> {
        match value.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.locals.get(&binding).copied(),
            _ => None,
        }
    }

    pub fn owns(&self, value: &TypedExpr) -> bool {
        self.locals
            .values()
            .any(|local| matches!(local, BoundLocal::Value(bound) if std::ptr::eq(*bound, value)))
    }

    fn collect(&mut self, expr: &'e TypedExpr) {
        self.bind(expr);
        for child in expr.children() {
            self.collect(child);
        }
    }

    fn bind(&mut self, expr: &'e TypedExpr) {
        match &expr.kind {
            TypedExprKind::Control(TypedControl::Block(block)) => self.lets(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => self.arms(matched),
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                self.loop_binders(body, arguments)
            }
            _ => {}
        }
    }

    fn arms(&mut self, matched: &'e TypedMatch) {
        for arm in &matched.arms {
            for binder in arm.pattern.binders() {
                self.locals
                    .insert(binder.binding, BoundLocal::Within(&matched.value));
            }
            self.lets(&arm.body);
        }
    }

    fn loop_binders(&mut self, body: &'e TypedLambda, arguments: &'e [TypedArgument]) {
        for (parameter, argument) in body.params.iter().zip(arguments) {
            self.locals
                .insert(parameter.binding, BoundLocal::Value(&argument.value));
        }
        self.lets(&body.body);
    }

    fn lets(&mut self, block: &'e TypedBlock) {
        for item in &block.items {
            if let TypedBodyItem::Let(binder, value) = item {
                self.locals.insert(binder.binding, BoundLocal::Value(value));
            }
        }
    }
}
