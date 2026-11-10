use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::BindingId;
use crate::types::{
    Pipelines, ReturnedValues, Type, TypedBlock, TypedBodyItem, TypedControl, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};

pub struct HeldBorrows<'a> {
    program: &'a TypedProgram,
}

struct ScopeLocal<'l> {
    ty: &'l Type,
    sources: FxHashSet<BindingId>,
}

#[derive(Default)]
struct FunctionScope<'l> {
    locals: FxHashMap<BindingId, ScopeLocal<'l>>,
}

impl<'a> HeldBorrows<'a> {
    pub fn new(program: &'a TypedProgram) -> Self {
        Self { program }
    }

    pub fn kept_parameters(&self, lambda: &TypedLambda) -> FxHashSet<BindingId> {
        let scope = FunctionScope::of(lambda);
        let mut held = Vec::new();
        for closure in ReturnedValues::new(self.program).closures(lambda) {
            self.collect_held(closure, &mut held);
        }
        let parameters: FxHashSet<BindingId> =
            lambda.params.iter().map(|param| param.binding).collect();
        self.sources(held, &scope)
            .intersection(&parameters)
            .copied()
            .collect()
    }

    fn collect_held(&self, expr: &TypedExpr, held: &mut Vec<BindingId>) {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                if self.may_hold_borrow(&expr.ty) {
                    held.push(*binding);
                }
            }
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.collect_held(child, held)),
        }
    }

    fn sources(&self, mut pending: Vec<BindingId>, scope: &FunctionScope) -> FxHashSet<BindingId> {
        let mut reached = FxHashSet::default();
        while let Some(binding) = pending.pop() {
            let Some(local) = scope.locals.get(&binding) else {
                continue;
            };
            for &source in &local.sources {
                if reached.insert(source) && self.follows(source, scope) {
                    pending.push(source);
                }
            }
        }
        reached
    }

    fn follows(&self, source: BindingId, scope: &FunctionScope) -> bool {
        scope
            .locals
            .get(&source)
            .is_some_and(|local| self.may_hold_borrow(local.ty))
    }

    fn may_hold_borrow(&self, ty: &Type) -> bool {
        matches!(ty, Type::Function(..))
            || self.program.type_holds_borrow(ty)
            || Pipelines::new(&self.program.rust_types).is_pipeline(ty)
    }
}

impl<'l> FunctionScope<'l> {
    fn of(lambda: &'l TypedLambda) -> Self {
        let mut scope = Self::default();
        scope.block(&lambda.body);
        scope
    }

    fn block(&mut self, block: &'l TypedBlock) {
        for item in &block.items {
            match item {
                TypedBodyItem::Let(binder, value) => {
                    self.bind(binder.binding, &binder.ty, value);
                    self.expr(value);
                }
                TypedBodyItem::Expr(expr) => self.expr(expr),
            }
        }
    }

    fn expr(&mut self, expr: &'l TypedExpr) {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                for (param, argument) in body.params.iter().zip(arguments) {
                    self.bind(param.binding, &param.ty, &argument.value);
                }
                self.block(&body.body);
                arguments
                    .iter()
                    .for_each(|argument| self.expr(&argument.value));
            }
            TypedExprKind::Control(TypedControl::Block(block)) => self.block(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                self.expr(&matched.value);
                for arm in &matched.arms {
                    for binder in arm.pattern.binders() {
                        self.bind(binder.binding, &binder.ty, &matched.value);
                    }
                    self.block(&arm.body);
                }
            }
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.expr(child)),
        }
    }

    fn bind(&mut self, binding: BindingId, ty: &'l Type, value: &TypedExpr) {
        let mut sources = FxHashSet::default();
        Self::names(value, &mut sources);
        self.locals.insert(binding, ScopeLocal { ty, sources });
    }

    fn names(expr: &TypedExpr, sources: &mut FxHashSet<BindingId>) {
        if let TypedExprKind::Leaf(TypedLeaf::Local(binding)) = &expr.kind {
            sources.insert(*binding);
        }
        expr.children()
            .into_iter()
            .for_each(|child| Self::names(child, sources));
    }
}

#[cfg(test)]
mod tests {
    use super::HeldBorrows;
    use crate::core::BindingId;
    use crate::types::{TypedExprKind, TypedLambda, TypedProgram, TypedSource};

    struct KeptFixture {
        program: TypedProgram,
    }

    impl KeptFixture {
        const COUNTERS: &str = "
            (defn looped! (items)
              (push! items 1)
              (loop (it (iter items))
                (fn () (count it))))
            (defn beside! (x items)
              (set! x (+ x 1))
              (local it (iter items))
              (fn () (count it)))
            (defn adder! (x)
              (set! x (+ x 1))
              (local g (fn (y) (+ y 1)))
              (fn (y) (g y)))
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TypedSource::expand(Self::COUNTERS).typed(),
            }
        }

        fn kept_names(&self, function: &str) -> Vec<&str> {
            let lambda = self.lambda(function);
            let kept = HeldBorrows::new(&self.program).kept_parameters(lambda);
            lambda
                .params
                .iter()
                .filter(|param| kept.contains(&param.binding))
                .map(|param| self.name(param.binding))
                .collect()
        }

        fn lambda(&self, function: &str) -> &TypedLambda {
            self.program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .filter(|definition| self.name(definition.binding) == function)
                .find_map(|definition| match &definition.value.kind {
                    TypedExprKind::Fn(lambda) => Some(lambda.as_ref()),
                    _ => None,
                })
                .expect("the function is defined")
        }

        fn name(&self, binding: BindingId) -> &str {
            self.program.bindings.get(binding).name()
        }
    }

    #[test]
    fn a_loop_binder_keeps_the_parameter_it_borrows() {
        assert_eq!(KeptFixture::new().kept_names("looped!"), vec!["items"]);
    }

    #[test]
    fn a_held_local_keeps_only_the_parameter_it_borrows() {
        assert_eq!(KeptFixture::new().kept_names("beside!"), vec!["items"]);
    }

    #[test]
    fn a_local_closure_that_captures_nothing_keeps_no_parameter() {
        assert!(KeptFixture::new().kept_names("adder!").is_empty());
    }
}
