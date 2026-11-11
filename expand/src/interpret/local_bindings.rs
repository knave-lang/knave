use crate::core::{BindingId, Block, BodyItem, Control, Expr, ExprKind, Lambda};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LocalBindings {
    first: u32,
    bits: Box<[u64]>,
}

impl LocalBindings {
    const WORD_BITS: usize = u64::BITS as usize;

    pub fn of_lambda(lambda: &Lambda) -> Self {
        let mut walk = BinderWalk::default();
        let mut pending = Vec::new();
        walk.bind_lambda(lambda, &mut pending);
        walk.collect(pending);
        Self::of_bindings(&walk.binders)
    }

    pub fn of_expr(expr: &Expr) -> Self {
        let mut walk = BinderWalk::default();
        walk.collect(vec![expr]);
        Self::of_bindings(&walk.binders)
    }

    #[inline]
    pub fn binds(&self, binding: BindingId) -> bool {
        let offset = binding.index().wrapping_sub(self.first) as usize;
        self.bits
            .get(offset / Self::WORD_BITS)
            .is_some_and(|word| word & Self::bit_of(offset) != 0)
    }

    fn of_bindings(bindings: &[BindingId]) -> Self {
        let indexes = bindings.iter().map(|binding| binding.index());
        let (Some(first), Some(last)) = (indexes.clone().min(), indexes.clone().max()) else {
            return Self::default();
        };
        let mut bits = vec![0; (last - first) as usize / Self::WORD_BITS + 1];
        for offset in indexes.map(|index| (index - first) as usize) {
            bits[offset / Self::WORD_BITS] |= Self::bit_of(offset);
        }
        Self {
            first,
            bits: bits.into_boxed_slice(),
        }
    }

    fn bit_of(offset: usize) -> u64 {
        1 << (offset % Self::WORD_BITS)
    }
}

#[derive(Default)]
struct BinderWalk {
    binders: Vec<BindingId>,
}

impl BinderWalk {
    fn collect<'e>(&mut self, mut pending: Vec<&'e Expr>) {
        while let Some(expr) = pending.pop() {
            self.visit(expr, &mut pending);
        }
    }

    fn visit<'e>(&mut self, expr: &'e Expr, pending: &mut Vec<&'e Expr>) {
        match expr.kind() {
            ExprKind::Leaf(_) => {}
            ExprKind::Call(callee, arguments) => {
                pending.push(callee);
                pending.extend(arguments);
            }
            ExprKind::Control(Control::If(test, then, otherwise)) => {
                pending.extend([test.as_ref(), then.as_ref(), otherwise.as_ref()]);
            }
            ExprKind::Fn(lambda) => self.bind_lambda(lambda, pending),
            ExprKind::Control(Control::Block(block)) => self.bind_block(block, pending),
            _ => pending.extend(expr.children()),
        }
    }

    fn bind_lambda<'e>(&mut self, lambda: &'e Lambda, pending: &mut Vec<&'e Expr>) {
        self.binders
            .extend(lambda.params.iter().map(|param| param.binding));
        self.bind_block(&lambda.body, pending);
    }

    fn bind_block<'e>(&mut self, block: &'e Block, pending: &mut Vec<&'e Expr>) {
        self.binders
            .extend(block.items.iter().filter_map(|item| match item {
                BodyItem::Let(binder, _) => Some(binder.binding),
                BodyItem::Expr(_) => None,
            }));
        pending.extend(block.expressions());
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

        fn deref_of(value: Expr) -> Expr {
            Expr::new(ExprKind::Deref(Box::new(value)), Self::span())
        }

        fn if_of(branch: Expr) -> Expr {
            let test = Box::new(Self::read(Self::GLOBAL));
            let otherwise = Box::new(Self::read(Self::GLOBAL));
            let choice = Control::If(test, Box::new(branch), otherwise);
            Expr::new(ExprKind::Control(choice), Self::span())
        }

        fn call_of(argument: Expr) -> Expr {
            let callee = Box::new(Self::read(Self::GLOBAL));
            Expr::new(ExprKind::Call(callee, vec![argument]), Self::span())
        }

        fn locals_at(indexes: &[u32]) -> LocalBindings {
            let bindings: Vec<BindingId> =
                indexes.iter().copied().map(BindingId::from_index).collect();
            LocalBindings::of_bindings(&bindings)
        }

        fn function() -> Lambda {
            let body = Block {
                items: vec![
                    BodyItem::Let(Self::binder(Self::LET), Self::read(Self::PARAMETER)),
                    BodyItem::Expr(Self::call_of(Self::if_of(Self::deref_of(
                        Self::nested_block(),
                    )))),
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

    #[test]
    fn a_name_between_two_locals_is_free() {
        let locals = LoweredFixture::locals_at(&[10, 12]);
        assert!(!locals.binds(BindingId::from_index(11)));
    }

    #[test]
    fn a_name_before_the_first_local_is_free() {
        let locals = LoweredFixture::locals_at(&[10, 12]);
        assert!(!locals.binds(BindingId::from_index(9)));
    }

    #[test]
    fn locals_words_apart_each_bind_alone() {
        let locals = LoweredFixture::locals_at(&[10, 140]);
        assert!(locals.binds(BindingId::from_index(10)));
        assert!(locals.binds(BindingId::from_index(140)));
        assert!(!locals.binds(BindingId::from_index(74)));
    }

    #[test]
    fn code_without_binders_leaves_every_name_free() {
        let locals = LocalBindings::of_expr(&LoweredFixture::read(LoweredFixture::GLOBAL));
        assert!(!locals.binds(LoweredFixture::GLOBAL));
    }
}
