use super::{
    AnnotationLowering, ChildTyping, ExpectedChildTyping, ExpressionForm, LocalBinding, LocalScope,
};
use crate::core::{Block, BodyItem};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;
use crate::types::typed::{TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind};

pub(in crate::types) struct BlockTyping<'e> {
    block: &'e Block,
    span: Span,
}

impl<'e> BlockTyping<'e> {
    pub(in crate::types) fn new(block: &'e Block, span: Span) -> Self {
        Self { block, span }
    }

    pub(in crate::types) fn type_block<C>(
        &self,
        context: &mut C,
        tail: Option<Expectation>,
    ) -> Result<(TypedBlock, Type), Diagnostic>
    where
        C: ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope,
    {
        let mut tail = tail;
        let mut items = Vec::new();
        let mut result = Type::unit();
        let last = self.block.items.len().checked_sub(1);
        for (position, item) in self.block.items.iter().enumerate() {
            let fitted = if Some(position) == last {
                tail.take()
            } else {
                None
            };
            let (typed, item_type) = Self::type_item(context, item, fitted)?;
            items.push(typed);
            result = item_type;
        }
        Ok((TypedBlock { items }, result))
    }

    fn type_item<C>(
        context: &mut C,
        item: &BodyItem,
        fitted: Option<Expectation>,
    ) -> Result<(TypedBodyItem, Type), Diagnostic>
    where
        C: ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope,
    {
        let typed = match (item, fitted) {
            (BodyItem::Let(binder, value), _) => {
                return LocalBinding::new(binder, value).bind(context);
            }
            (BodyItem::Expr(expr), Some(expectation)) => {
                context.type_child_against(expr, expectation)?
            }
            (BodyItem::Expr(expr), None) => context.type_child(expr)?,
        };
        let ty = typed.ty.clone();
        Ok((TypedBodyItem::Expr(typed), ty))
    }
}

impl<C> ExpressionForm<C> for BlockTyping<'_>
where
    C: ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope,
{
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let (block, ty) = self.type_block(context, None)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Control(TypedControl::Block(block)),
            ty,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::BlockTyping;
    use crate::types::expectation::Expectation;
    use crate::types::forms::body_fixture::BodyFixture;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::ty::Type;

    #[test]
    fn a_block_has_the_type_of_its_last_item() {
        let block = BodyFixture::block(vec![BodyFixture::truth(), BodyFixture::text()]);
        let typed = BlockTyping::new(&block, FormFixtures::span())
            .type_block(&mut BodyFixture::default(), None)
            .map(|(_, ty)| ty)
            .ok();
        assert_eq!(typed, Some(BodyFixture::text_type()));
    }

    #[test]
    fn only_the_last_item_meets_the_expectation() {
        let block = BodyFixture::block(vec![BodyFixture::truth(), BodyFixture::truth()]);
        let mut fixture = BodyFixture::default();
        let expected = BodyFixture::text_type();
        let _ = BlockTyping::new(&block, FormFixtures::span())
            .type_block(&mut fixture, Some(Expectation::plain(&expected)));
        assert_eq!(fixture.fitted, [expected]);
    }

    #[test]
    fn a_block_that_ends_in_a_let_is_unit() {
        let block = BodyFixture::block(vec![BodyFixture::truth(), BodyFixture::binding(None)]);
        let typed = BlockTyping::new(&block, FormFixtures::span())
            .type_block(&mut BodyFixture::default(), None)
            .map(|(_, ty)| ty)
            .ok();
        assert_eq!(typed, Some(Type::unit()));
    }
}
