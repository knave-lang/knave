use super::{BindingId, Control, Expr, ExprKind, Leaf, TypeExpr};

#[derive(Default)]
pub struct WrittenTypes {
    named: Vec<BindingId>,
}

impl WrittenTypes {
    pub fn named(&self) -> &[BindingId] {
        &self.named
    }

    pub(super) fn note(&mut self, expr: &Expr) {
        match expr.kind() {
            ExprKind::Leaf(Leaf::Rust(template)) => {
                let clauses = template.clauses.iter().map(|clause| &clause.target);
                self.name_all(std::iter::once(&template.type_expr).chain(clauses));
            }
            ExprKind::Fn(lambda) => self.name_all(lambda.annotations()),
            ExprKind::Control(Control::Block(block)) => self.name_all(block.annotations()),
            ExprKind::Control(Control::Match(_, arms)) => {
                self.name_all(arms.iter().flat_map(|arm| arm.body.annotations()));
            }
            _ => {}
        }
    }

    fn name_all<'t>(&mut self, written: impl Iterator<Item = &'t TypeExpr>) {
        for type_expr in written {
            self.named.extend(type_expr.named_bindings());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::core::References;
    use crate::core::{
        Arm, Binder, BindingArena, BindingId, Block, BodyItem, Control, Expr, ExprKind, Lambda,
        Leaf, Literal, Pattern, PatternKind, RustClause, RustTemplate, TypeExpr, TypeExprKind,
    };
    use crate::source::{SourceId, Span};

    struct WrittenTypesFixture;

    impl WrittenTypesFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn named(index: u32) -> TypeExpr {
            TypeExpr::new(
                TypeExprKind::Named(BindingId::from_index(index), Vec::new()),
                Self::span(),
            )
        }

        fn binder(index: u32, annotation: TypeExpr) -> Binder {
            Binder {
                binding: BindingId::from_index(index),
                annotation: Some(annotation),
                span: Self::span(),
            }
        }

        fn expr(kind: ExprKind) -> Expr {
            Expr::new(kind, Self::span())
        }

        fn typed(value: &Expr) -> Vec<u32> {
            References::of(value, &BindingArena::default())
                .written_types()
                .named()
                .iter()
                .map(|binding| binding.index())
                .collect()
        }
    }

    #[test]
    fn a_template_names_the_types_it_is_written_with() {
        let template = RustTemplate {
            template: "{0}.len()".into(),
            type_expr: WrittenTypesFixture::named(1),
            clauses: Vec::new(),
        };
        let value = WrittenTypesFixture::expr(ExprKind::Leaf(Leaf::Rust(Rc::new(template))));
        assert_eq!(WrittenTypesFixture::typed(&value), vec![1]);
    }

    #[test]
    fn a_function_names_its_parameter_and_local_annotations() {
        let local = WrittenTypesFixture::expr(ExprKind::Leaf(Leaf::Literal(Literal::Bool(true))));
        let body = Block {
            items: vec![BodyItem::Let(
                WrittenTypesFixture::binder(20, WrittenTypesFixture::named(2)),
                local,
            )],
        };
        let lambda = Lambda::new(
            vec![WrittenTypesFixture::binder(
                10,
                WrittenTypesFixture::named(1),
            )],
            body,
        );
        let value = WrittenTypesFixture::expr(ExprKind::Fn(Rc::new(lambda)));
        assert_eq!(WrittenTypesFixture::typed(&value), vec![1, 2]);
    }

    #[test]
    fn a_block_and_a_match_arm_name_their_local_annotations() {
        let local = |index| {
            BodyItem::Let(
                WrittenTypesFixture::binder(index * 10, WrittenTypesFixture::named(index)),
                WrittenTypesFixture::expr(ExprKind::Leaf(Leaf::Literal(Literal::Bool(true)))),
            )
        };
        let block = WrittenTypesFixture::expr(ExprKind::Control(Control::Block(Block {
            items: vec![local(1)],
        })));
        let arm = Arm {
            pattern: Pattern::new(PatternKind::Wildcard, WrittenTypesFixture::span()),
            body: Block {
                items: vec![local(2)],
            },
            span: WrittenTypesFixture::span(),
        };
        let matched = WrittenTypesFixture::expr(ExprKind::Control(Control::Match(
            Box::new(block),
            vec![arm],
        )));
        assert_eq!(WrittenTypesFixture::typed(&matched), vec![2, 1]);
    }

    #[test]
    fn a_template_clause_names_its_target() {
        let template = RustTemplate {
            template: "{0}.len()".into(),
            type_expr: WrittenTypesFixture::named(1),
            clauses: vec![RustClause {
                target: WrittenTypesFixture::named(3),
                template: "{0}.size()".into(),
            }],
        };
        let value = WrittenTypesFixture::expr(ExprKind::Leaf(Leaf::Rust(Rc::new(template))));
        assert_eq!(WrittenTypesFixture::typed(&value), vec![1, 3]);
    }
}
