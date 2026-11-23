use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::{
    Borrow, TypedArgument, TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedMatch, TypedProgram,
};

pub(super) struct PassedLocal {
    pub callee: BindingId,
    pub position: usize,
    pub local: BindingId,
}

pub(super) struct LocalBorrowCalls<'a> {
    program: &'a TypedProgram,
    outliving: FxHashSet<BindingId>,
    pub callees: FxHashSet<BindingId>,
    pub passed: Vec<PassedLocal>,
}

impl<'a> LocalBorrowCalls<'a> {
    pub(super) fn of(program: &'a TypedProgram, lambda: &TypedLambda) -> Self {
        let outliving = lambda
            .params
            .iter()
            .filter(|param| program.type_holds_borrow(&param.ty))
            .map(|param| param.binding)
            .collect();
        let mut calls = Self {
            program,
            outliving,
            callees: FxHashSet::default(),
            passed: Vec::new(),
        };
        calls.block(&lambda.body);
        calls
    }

    fn block(&mut self, block: &TypedBlock) {
        for item in &block.items {
            match item {
                TypedBodyItem::Let(binder, value) => self.binding(binder, value),
                TypedBodyItem::Expr(expr) => self.expr(expr),
            }
        }
    }

    fn binding(&mut self, binder: &TypedBinder, value: &TypedExpr) {
        self.expr(value);
        if self.program.type_holds_borrow(&binder.ty) && self.outlives_body(value) {
            self.outliving.insert(binder.binding);
        }
    }

    fn expr(&mut self, expr: &TypedExpr) {
        if let TypedExprKind::Call(callee, arguments) = &expr.kind {
            self.call(callee, arguments);
        }
        match &expr.kind {
            TypedExprKind::Fn(lambda) => self.block(&lambda.body),
            TypedExprKind::Control(TypedControl::Block(block)) => self.block(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => self.matched(matched),
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.expr(child)),
        }
    }

    fn call(&mut self, callee: &TypedExpr, arguments: &[TypedArgument]) {
        match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding))
                if !self.arguments_outlive_body(arguments) =>
            {
                self.callees.insert(binding);
            }
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => self.pass(binding, arguments),
            _ => {}
        }
    }

    fn pass(&mut self, callee: BindingId, arguments: &[TypedArgument]) {
        for (position, argument) in arguments.iter().enumerate() {
            if let TypedExprKind::Leaf(TypedLeaf::Local(local)) = argument.value.kind
                && self
                    .program
                    .is_function_returning_borrow(&argument.value.ty)
            {
                self.passed.push(PassedLocal {
                    callee,
                    position,
                    local,
                });
            }
        }
    }

    fn matched(&mut self, matched: &TypedMatch) {
        self.expr(&matched.value);
        for arm in &matched.arms {
            self.block(&arm.body);
        }
    }

    fn arguments_outlive_body(&self, arguments: &[TypedArgument]) -> bool {
        arguments
            .iter()
            .all(|argument| self.argument_outlives_body(argument))
    }

    fn argument_outlives_body(&self, argument: &TypedArgument) -> bool {
        if self.program.type_holds_borrow(&argument.value.ty) {
            self.outlives_body(&argument.value)
        } else {
            argument.borrow == Borrow::None
        }
    }

    fn outlives_body(&self, expr: &TypedExpr) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Literal(_) | TypedLeaf::Global(_)) => true,
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.outliving.contains(binding),
            TypedExprKind::Call(_, arguments) => self.arguments_outlive_body(arguments),
            TypedExprKind::Control(control) => self.control_outlives_body(control),
            _ => false,
        }
    }

    fn control_outlives_body(&self, control: &TypedControl) -> bool {
        match control {
            TypedControl::If(_, then, otherwise) => {
                self.outlives_body(then) && self.outlives_body(otherwise)
            }
            TypedControl::Match(matched) => matched
                .arms
                .iter()
                .all(|arm| self.block_outlives_body(&arm.body)),
            TypedControl::Block(block) => self.block_outlives_body(block),
        }
    }

    fn block_outlives_body(&self, block: &TypedBlock) -> bool {
        matches!(block.items.last(), Some(TypedBodyItem::Expr(tail)) if self.outlives_body(tail))
    }
}
