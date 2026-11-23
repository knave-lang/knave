use crate::source::Span;

use super::typed::{TypedBlock, TypedControl, TypedExpr, TypedExprKind};

pub(super) struct ResultSpan;

impl ResultSpan {
    pub(super) fn of(expr: &TypedExpr) -> Span {
        match &expr.kind {
            TypedExprKind::Control(TypedControl::Block(block)) => {
                Self::of_block(block).unwrap_or(expr.span)
            }
            _ => expr.span,
        }
    }

    pub(super) fn of_block(block: &TypedBlock) -> Option<Span> {
        block.tail().map(Self::of)
    }
}

#[cfg(test)]
mod tests {
    use super::ResultSpan;
    use crate::core::{BindingId, Literal};
    use crate::source::{SourceId, Span};
    use crate::types::Type;
    use crate::types::typed::{
        TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLeaf,
    };

    struct ResultSpanFixture;

    impl ResultSpanFixture {
        fn literal(at: usize) -> TypedExpr {
            TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                ty: Type::bool(),
                span: Self::span(at),
            }
        }

        fn block(items: Vec<TypedBodyItem>) -> TypedBlock {
            TypedBlock { items }
        }

        fn block_expr(block: TypedBlock, at: usize) -> TypedExpr {
            TypedExpr {
                kind: TypedExprKind::Control(TypedControl::Block(block)),
                ty: Type::bool(),
                span: Self::span(at),
            }
        }

        fn binding(at: usize) -> TypedBodyItem {
            let binder = TypedBinder {
                binding: BindingId::from_index(0),
                ty: Type::bool(),
                span: Self::span(at),
                annotated: false,
                written_view: false,
            };
            TypedBodyItem::Let(binder, Self::literal(at))
        }

        fn span(at: usize) -> Span {
            Span::new(SourceId::new(0), at..at + 1)
        }
    }

    #[test]
    fn a_block_results_where_its_last_expression_is() {
        let block =
            ResultSpanFixture::block(vec![TypedBodyItem::Expr(ResultSpanFixture::literal(4))]);
        assert_eq!(
            ResultSpan::of_block(&block),
            Some(ResultSpanFixture::span(4))
        );
    }

    #[test]
    fn a_block_expression_results_where_its_last_expression_is() {
        let block =
            ResultSpanFixture::block(vec![TypedBodyItem::Expr(ResultSpanFixture::literal(4))]);
        let expr = ResultSpanFixture::block_expr(block, 1);
        assert_eq!(ResultSpan::of(&expr), ResultSpanFixture::span(4));
    }

    #[test]
    fn a_block_that_ends_in_a_binding_has_no_result() {
        let block = ResultSpanFixture::block(vec![ResultSpanFixture::binding(4)]);
        assert_eq!(ResultSpan::of_block(&block), None);
    }
}
