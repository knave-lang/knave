use super::{ChildTyping, ExpressionForm, PlaceReads};
use crate::core::{Expr, ReferenceKind};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::problems::TypeProblems;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind};

pub(in crate::types) struct Borrowing<'e> {
    kind: ReferenceKind,
    value: &'e Expr,
    span: Span,
}

impl<'e> Borrowing<'e> {
    pub(in crate::types) fn new(kind: ReferenceKind, value: &'e Expr, span: Span) -> Self {
        Self { kind, value, span }
    }
}

impl<C> ExpressionForm<C> for Borrowing<'_>
where
    C: ChildTyping + PlaceReads,
{
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = context.type_child(self.value)?;
        if self.kind == ReferenceKind::Mutable && !value.is_place() {
            return Err(TypeProblems::mut_borrow_of_copy(
                value.span,
                context.field_place(&value),
            ));
        }
        let ty = match self.kind {
            ReferenceKind::Shared => Type::shared(context.read_type(&value)),
            ReferenceKind::Mutable => Type::mutable(context.changed_type(&value)),
        };
        Ok(TypedExpr {
            kind: TypedExprKind::Borrow(self.kind, Box::new(value)),
            ty,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Borrowing;
    use crate::core::{BindingId, Expr, ExprKind, Leaf, Literal, PrimitiveType, ReferenceKind};
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::forms::{ChildTyping, ExpressionForm, PlaceReads};
    use crate::types::ty::Type;
    use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};

    struct BorrowingFixture;

    impl BorrowingFixture {
        fn borrow(&mut self, kind: ReferenceKind, value: &Expr) -> Result<TypedExpr, Diagnostic> {
            Borrowing::new(kind, value, Self::span()).type_against(self, None)
        }

        fn local() -> Expr {
            Expr::new(
                ExprKind::Leaf(Leaf::Var(BindingId::from_index(0))),
                Self::span(),
            )
        }

        fn value() -> Expr {
            let text = Literal::String(Rc::from("x"));
            Expr::new(ExprKind::Leaf(Leaf::Literal(text)), Self::span())
        }

        fn read_type() -> Type {
            Type::Primitive(PrimitiveType::Str)
        }

        fn changed_type() -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }
    }

    impl ChildTyping for BorrowingFixture {
        fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
            let kind = match child.kind() {
                ExprKind::Leaf(Leaf::Var(binding)) => TypedLeaf::Local(*binding),
                ExprKind::Leaf(Leaf::Literal(literal)) => TypedLeaf::Literal(literal.clone()),
                _ => panic!("the fixture types leaves only"),
            };
            Ok(TypedExpr {
                kind: TypedExprKind::Leaf(kind),
                ty: Self::read_type(),
                span: child.span(),
            })
        }
    }

    impl PlaceReads for BorrowingFixture {
        fn read_type(&self, _value: &TypedExpr) -> Type {
            Self::read_type()
        }

        fn changed_type(&self, _value: &TypedExpr) -> Type {
            Self::changed_type()
        }

        fn field_place(&self, _value: &TypedExpr) -> Option<String> {
            None
        }
    }

    #[test]
    fn a_mut_borrow_of_a_value_is_refused() {
        let borrowed = BorrowingFixture.borrow(ReferenceKind::Mutable, &BorrowingFixture::value());
        assert!(borrowed.is_err());
    }

    #[test]
    fn a_shared_borrow_of_a_value_reads_its_type() {
        let borrowed = BorrowingFixture.borrow(ReferenceKind::Shared, &BorrowingFixture::value());
        let ty = borrowed.map(|typed| typed.ty).ok();
        assert_eq!(ty, Some(Type::shared(BorrowingFixture::read_type())));
    }

    #[test]
    fn a_mut_borrow_of_a_place_takes_its_changed_type() {
        let borrowed = BorrowingFixture.borrow(ReferenceKind::Mutable, &BorrowingFixture::local());
        let ty = borrowed.map(|typed| typed.ty).ok();
        assert_eq!(ty, Some(Type::mutable(BorrowingFixture::changed_type())));
    }
}
