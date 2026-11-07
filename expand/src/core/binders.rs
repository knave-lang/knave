use super::{Arm, BindingId, Block, BodyItem, Control, Expr, ExprKind, Pattern, PatternKind};

#[derive(Default)]
pub struct Binders {
    bound: Vec<BindingId>,
}

impl Binders {
    pub fn of(value: &Expr) -> Vec<BindingId> {
        let mut binders = Self::default();
        binders.collect(value);
        binders.bound
    }

    fn collect(&mut self, expr: &Expr) {
        match expr.kind() {
            ExprKind::Fn(lambda) => {
                self.bound
                    .extend(lambda.params.iter().map(|param| param.binding));
                self.block(&lambda.body);
            }
            ExprKind::Control(Control::Block(block)) => self.block(block),
            ExprKind::Control(Control::Match(_, arms)) => arms.iter().for_each(|arm| self.arm(arm)),
            _ => {}
        }
        for child in expr.children() {
            self.collect(child);
        }
    }

    fn block(&mut self, block: &Block) {
        for item in &block.items {
            if let BodyItem::Let(binder, _) = item {
                self.bound.push(binder.binding);
            }
        }
    }

    fn arm(&mut self, arm: &Arm) {
        self.pattern(&arm.pattern);
        self.block(&arm.body);
    }

    fn pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Bind(binding) | PatternKind::BareName { binding, .. } => {
                self.bound.push(*binding);
            }
            _ => {}
        }
        for nested in pattern.nested() {
            self.pattern(nested);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Binders;
    use crate::core::{
        Arm, Binder, BindingId, Block, BodyItem, Control, Expr, ExprKind, Lambda, Leaf, Literal,
        Pattern, PatternArguments, PatternKind, PatternPart,
    };
    use crate::source::{SourceId, Span};
    use std::rc::Rc;

    struct BinderFixture;

    impl BinderFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn unit() -> Expr {
            Expr::new(
                ExprKind::Leaf(Leaf::Literal(Literal::Bool(true))),
                Self::span(),
            )
        }

        fn binder(index: u32) -> Binder {
            Binder {
                binding: BindingId::from_index(index),
                annotation: None,
                span: Self::span(),
            }
        }

        fn letting(index: u32, body: Expr) -> Block {
            Block {
                items: vec![
                    BodyItem::Let(Self::binder(index), Self::unit()),
                    BodyItem::Expr(body),
                ],
            }
        }

        fn function(param: u32, body: Block) -> Expr {
            let lambda = Lambda {
                params: vec![Self::binder(param)],
                body,
            };
            Expr::new(ExprKind::Fn(Rc::new(lambda)), Self::span())
        }

        fn matching(bound: u32) -> Expr {
            let pattern = Pattern::new(
                PatternKind::Bind(BindingId::from_index(bound)),
                Self::span(),
            );
            let arm = Arm {
                pattern,
                body: Self::letting(bound + 1, Self::unit()),
                span: Self::span(),
            };
            let control = Control::Match(Box::new(Self::unit()), vec![arm]);
            Expr::new(ExprKind::Control(control), Self::span())
        }

        fn indices(value: &Expr) -> Vec<u32> {
            let mut found: Vec<u32> = Binders::of(value)
                .into_iter()
                .map(BindingId::index)
                .collect();
            found.sort_unstable();
            found
        }
    }

    #[test]
    fn a_function_binds_its_parameters_and_its_lets() {
        let function = BinderFixture::function(1, BinderFixture::letting(2, BinderFixture::unit()));
        assert_eq!(BinderFixture::indices(&function), vec![1, 2]);
    }

    #[test]
    fn a_match_arm_binds_its_pattern_and_its_lets() {
        let function =
            BinderFixture::function(1, BinderFixture::letting(2, BinderFixture::matching(3)));
        assert_eq!(BinderFixture::indices(&function), vec![1, 2, 3, 4]);
    }

    #[test]
    fn a_block_inside_a_body_binds_its_lets() {
        let inner = BinderFixture::letting(3, BinderFixture::unit());
        let block = Expr::new(
            ExprKind::Control(Control::Block(inner)),
            BinderFixture::span(),
        );
        let function = BinderFixture::function(
            1,
            Block {
                items: vec![BodyItem::Expr(block)],
            },
        );
        assert_eq!(BinderFixture::indices(&function), vec![1, 3]);
    }

    #[test]
    fn a_nested_pattern_binds_its_names() {
        let span = BinderFixture::span();
        let inner = Pattern::new(PatternKind::Bind(BindingId::from_index(5)), span);
        let parts = vec![PatternPart::Positional(inner)];
        let outer = PatternKind::Variant(BindingId::from_index(9), PatternArguments { parts });
        let arm = Arm {
            pattern: Pattern::new(outer, span),
            body: Block::default(),
            span,
        };
        let control = Control::Match(Box::new(BinderFixture::unit()), vec![arm]);
        let matching = Expr::new(ExprKind::Control(control), span);
        assert_eq!(BinderFixture::indices(&matching), vec![5]);
    }

    #[test]
    fn a_bare_name_pattern_binds_its_name() {
        let span = BinderFixture::span();
        let kind = PatternKind::BareName {
            name: "x".into(),
            binding: BindingId::from_index(6),
            candidates: vec![BindingId::from_index(9)],
            outer: None,
        };
        let arm = Arm {
            pattern: Pattern::new(kind, span),
            body: Block::default(),
            span,
        };
        let control = Control::Match(Box::new(BinderFixture::unit()), vec![arm]);
        let matching = Expr::new(ExprKind::Control(control), span);
        assert_eq!(BinderFixture::indices(&matching), vec![6]);
    }

    #[test]
    fn a_plain_value_binds_nothing() {
        assert!(Binders::of(&BinderFixture::unit()).is_empty());
    }
}
