use std::hint::black_box;
use std::rc::Rc;

use super::form_fixtures::FormFixtures;
use super::{
    AnnotationLowering, ChildTyping, ExpectedChildTyping, ExpressionForm, LocalScope, Sequence,
};
use crate::core::{
    Binder, BindingId, Block, BodyItem, Control, Expr, ExprKind, Leaf, Literal, PrimitiveType,
    TypeExpr, TypeExprKind,
};
use crate::diagnostic::Diagnostic;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

#[derive(Default)]
pub(in crate::types) struct BodyFixture {
    pub(in crate::types) fitted: Vec<Type>,
    pub(in crate::types) bound: Vec<(BindingId, Type)>,
    pub(in crate::types) stack_marks: Vec<usize>,
}

impl BodyFixture {
    pub(in crate::types) const LOCAL: BindingId = BindingId::from_index(0);

    pub(in crate::types) fn truth() -> BodyItem {
        BodyItem::Expr(FormFixtures::leaf(Literal::Bool(true)))
    }

    pub(in crate::types) fn text() -> BodyItem {
        BodyItem::Expr(FormFixtures::leaf(Literal::String(Rc::from("x"))))
    }

    pub(in crate::types) fn binding(annotation: Option<PrimitiveType>) -> BodyItem {
        BodyItem::Let(
            Self::binder(annotation),
            FormFixtures::leaf(Literal::Bool(true)),
        )
    }

    pub(in crate::types) fn binder(annotation: Option<PrimitiveType>) -> Binder {
        let annotation = annotation.map(|primitive| {
            TypeExpr::new(TypeExprKind::Primitive(primitive), FormFixtures::span())
        });
        Binder {
            binding: Self::LOCAL,
            annotation,
            span: FormFixtures::span(),
        }
    }

    pub(in crate::types) fn nested_do_forms(depth: usize) -> Block {
        (0..depth).fold(Self::block(vec![Self::truth()]), |inner, _| {
            let do_form = Expr::new(
                ExprKind::Control(Control::Block(inner)),
                FormFixtures::span(),
            );
            Self::block(vec![BodyItem::Expr(do_form)])
        })
    }

    fn stack_mark() -> usize {
        let mark = black_box(0u8);
        std::ptr::from_ref(&mark).addr()
    }

    pub(in crate::types) fn block(items: Vec<BodyItem>) -> Block {
        Block { items }
    }

    pub(in crate::types) fn text_type() -> Type {
        Type::Primitive(PrimitiveType::Str)
    }

    fn type_of(child: &Expr) -> Type {
        match child.kind() {
            ExprKind::Leaf(Leaf::Literal(Literal::Bool(_))) => Type::bool(),
            _ => Self::text_type(),
        }
    }
}

impl ChildTyping for BodyFixture {
    fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.stack_marks.push(Self::stack_mark());
        match child.kind() {
            ExprKind::Control(Control::Block(block)) => {
                Sequence::new(block, child.span()).type_against(self, None)
            }
            _ => Ok(FormFixtures::typed_leaf(
                Literal::Bool(true),
                Self::type_of(child),
            )),
        }
    }
}

impl ExpectedChildTyping for BodyFixture {
    fn type_child_against(
        &mut self,
        _child: &Expr,
        expectation: Expectation,
    ) -> Result<TypedExpr, Diagnostic> {
        self.fitted.push(expectation.expected.clone());
        Ok(FormFixtures::typed_leaf(
            Literal::Bool(true),
            expectation.expected.clone(),
        ))
    }
}

impl AnnotationLowering for BodyFixture {
    fn lower_annotation(&mut self, annotation: &TypeExpr) -> Result<Type, Diagnostic> {
        let TypeExprKind::Primitive(primitive) = annotation.kind() else {
            panic!("the fixture lowers primitives only")
        };
        Ok(Type::Primitive(*primitive))
    }
}

impl LocalScope for BodyFixture {
    fn bind_local(&mut self, binding: BindingId, ty: Type) {
        self.bound.push((binding, ty));
    }
}
