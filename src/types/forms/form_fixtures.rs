use crate::core::{Expr, ExprKind, Leaf, Literal};
use crate::source::{SourceId, Span};
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};

pub(in crate::types) struct FormFixtures;

impl FormFixtures {
    pub(in crate::types) fn span() -> Span {
        Self::span_at(0)
    }

    pub(in crate::types) fn span_at(at: usize) -> Span {
        Span::new(SourceId::new(0), at..at + 1)
    }

    pub(in crate::types) fn leaf(literal: Literal) -> Expr {
        Self::leaf_at(literal, 0)
    }

    pub(in crate::types) fn leaf_at(literal: Literal, at: usize) -> Expr {
        Expr::new(ExprKind::Leaf(Leaf::Literal(literal)), Self::span_at(at))
    }

    pub(in crate::types) fn typed_leaf(literal: Literal, ty: Type) -> TypedExpr {
        TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Literal(literal)),
            ty,
            span: Self::span(),
        }
    }
}
