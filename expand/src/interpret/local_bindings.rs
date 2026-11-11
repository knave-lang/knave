use rustc_hash::FxHashSet;

use crate::core::{BindingId, Block, BodyItem, Control, Expr, ExprKind, Lambda};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LocalBindings {
    bound: FxHashSet<BindingId>,
}

impl LocalBindings {
    pub fn of_lambda(lambda: &Lambda) -> Self {
        let mut locals = Self::default();
        let body = locals.bind_lambda(lambda);
        locals.collect(body);
        locals
    }

    pub fn of_expr(expr: &Expr) -> Self {
        let mut locals = Self::default();
        locals.collect(vec![expr]);
        locals
    }

    pub fn binds(&self, binding: BindingId) -> bool {
        self.bound.contains(&binding)
    }

    fn collect(&mut self, mut pending: Vec<&Expr>) {
        while let Some(expr) = pending.pop() {
            let children = self.bind(expr);
            pending.extend(children);
        }
    }

    fn bind<'e>(&mut self, expr: &'e Expr) -> Vec<&'e Expr> {
        match expr.kind() {
            ExprKind::Fn(lambda) => self.bind_lambda(lambda),
            ExprKind::Control(Control::Block(block)) => self.bind_block(block),
            _ => expr.children(),
        }
    }

    fn bind_lambda<'e>(&mut self, lambda: &'e Lambda) -> Vec<&'e Expr> {
        self.bound
            .extend(lambda.params.iter().map(|param| param.binding));
        self.bind_block(&lambda.body)
    }

    fn bind_block<'e>(&mut self, block: &'e Block) -> Vec<&'e Expr> {
        self.bound
            .extend(block.items.iter().filter_map(|item| match item {
                BodyItem::Let(binder, _) => Some(binder.binding),
                BodyItem::Expr(_) => None,
            }));
        block.expressions().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::LocalBindings;
    use crate::core::{Binder, BindingId, Block, BodyItem, Control, Expr, ExprKind, Lambda, Leaf};
    use crate::source::{SourceId, Span};

    struct LoweredFixture;

    impl LoweredFixture {
        const PARAMETER: BindingId = BindingId::from_index(1);
        const LET: BindingId = BindingId::from_index(2);
        const NESTED_PARAMETER: BindingId = BindingId::from_index(3);
        const NESTED_LET: BindingId = BindingId::from_index(4);
        const GLOBAL: BindingId = BindingId::from_index(5);

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn binder(binding: BindingId) -> Binder {
            Binder {
                binding,
                annotation: None,
                span: Self::span(),
            }
        }

        fn read(binding: BindingId) -> Expr {
            Expr::new(ExprKind::Leaf(Leaf::Var(binding)), Self::span())
        }

        fn nested_function() -> Expr {
            let body = Block {
                items: vec![BodyItem::Expr(Self::read(Self::GLOBAL))],
            };
            let lambda = Lambda::new(vec![Self::binder(Self::NESTED_PARAMETER)], body);
            Expr::new(ExprKind::Fn(Rc::new(lambda)), Self::span())
        }

        fn nested_block() -> Expr {
            let block = Block {
                items: vec![BodyItem::Let(
                    Self::binder(Self::NESTED_LET),
                    Self::nested_function(),
                )],
            };
            Expr::new(ExprKind::Control(Control::Block(block)), Self::span())
        }

        fn call_of(argument: Expr) -> Expr {
            let callee = Box::new(Self::read(Self::GLOBAL));
            Expr::new(ExprKind::Call(callee, vec![argument]), Self::span())
        }

        fn function() -> Lambda {
            let body = Block {
                items: vec![
                    BodyItem::Let(Self::binder(Self::LET), Self::read(Self::PARAMETER)),
                    BodyItem::Expr(Self::call_of(Self::nested_block())),
                ],
            };
            Lambda::new(vec![Self::binder(Self::PARAMETER)], body)
        }
    }

    #[test]
    fn a_function_binds_its_parameters_and_lets_at_every_depth() {
        let locals = LocalBindings::of_lambda(&LoweredFixture::function());
        let bound = [
            LoweredFixture::PARAMETER,
            LoweredFixture::LET,
            LoweredFixture::NESTED_PARAMETER,
            LoweredFixture::NESTED_LET,
        ];
        assert!(bound.iter().all(|binding| locals.binds(*binding)));
    }

    #[test]
    fn a_function_leaves_a_global_free() {
        let locals = LocalBindings::of_lambda(&LoweredFixture::function());
        assert!(!locals.binds(LoweredFixture::GLOBAL));
    }

    #[test]
    fn an_expression_binds_the_lets_and_parameters_inside_it() {
        let locals = LocalBindings::of_expr(&LoweredFixture::nested_block());
        assert!(locals.binds(LoweredFixture::NESTED_LET));
        assert!(locals.binds(LoweredFixture::NESTED_PARAMETER));
        assert!(!locals.binds(LoweredFixture::GLOBAL));
    }
}
