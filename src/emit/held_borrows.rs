use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::{
    Pipelines, ReturnedValues, Type, TypedBlock, TypedBodyItem, TypedControl, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};

pub struct HeldBorrows<'a> {
    program: &'a TypedProgram,
}

#[derive(Default)]
struct FunctionScope {
    locals: FxHashSet<BindingId>,
}

impl<'a> HeldBorrows<'a> {
    pub fn new(program: &'a TypedProgram) -> Self {
        Self { program }
    }

    pub fn in_returned_closures(&self, lambda: &TypedLambda) -> bool {
        let scope = FunctionScope::of(lambda);
        ReturnedValues::new(self.program)
            .closures(lambda)
            .into_iter()
            .any(|closure| self.holds(closure, &scope))
    }

    fn holds(&self, expr: &TypedExpr, scope: &FunctionScope) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                scope.locals.contains(binding) && self.may_hold_borrow(&expr.ty)
            }
            _ => expr
                .children()
                .into_iter()
                .any(|child| self.holds(child, scope)),
        }
    }

    fn may_hold_borrow(&self, ty: &Type) -> bool {
        matches!(ty, Type::Function(..))
            || self.program.type_holds_borrow(ty)
            || Pipelines::new(&self.program.rust_types).is_pipeline(ty)
    }
}

impl FunctionScope {
    fn of(lambda: &TypedLambda) -> Self {
        let mut scope = Self::default();
        scope.block(&lambda.body);
        scope
    }

    fn block(&mut self, block: &TypedBlock) {
        for item in &block.items {
            match item {
                TypedBodyItem::Let(binder, value) => {
                    self.locals.insert(binder.binding);
                    self.expr(value);
                }
                TypedBodyItem::Expr(expr) => self.expr(expr),
            }
        }
    }

    fn expr(&mut self, expr: &TypedExpr) {
        match &expr.kind {
            TypedExprKind::Fn(_) => {}
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                self.locals
                    .extend(body.params.iter().map(|param| param.binding));
                self.block(&body.body);
                arguments
                    .iter()
                    .for_each(|argument| self.expr(&argument.value));
            }
            TypedExprKind::Control(TypedControl::Block(block)) => self.block(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                self.expr(&matched.value);
                for arm in &matched.arms {
                    self.locals
                        .extend(arm.pattern.binders().iter().map(|binder| binder.binding));
                    self.block(&arm.body);
                }
            }
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.expr(child)),
        }
    }
}
