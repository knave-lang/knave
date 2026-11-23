use super::{AnnotationLowering, ChildTyping, ExpectedChildTyping, LocalBinding, LocalScope};
use crate::core::{Block, BodyItem};
use crate::diagnostic::Diagnostic;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;
use crate::types::typed::{TypedBlock, TypedBodyItem};

pub(in crate::types) struct Body<'e> {
    block: &'e Block,
}

impl<'e> Body<'e> {
    pub(in crate::types) fn new(block: &'e Block) -> Self {
        Self { block }
    }

    pub(in crate::types) fn type_with<C>(
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

#[cfg(test)]
mod tests {
    use super::Body;
    use crate::types::expectation::Expectation;
    use crate::types::forms::body_fixture::BodyFixture;
    use crate::types::ty::Type;

    #[test]
    fn a_body_has_the_type_of_its_last_item() {
        let block = BodyFixture::block(vec![BodyFixture::truth(), BodyFixture::text()]);
        let typed = Body::new(&block)
            .type_with(&mut BodyFixture::default(), None)
            .map(|(_, ty)| ty)
            .ok();
        assert_eq!(typed, Some(BodyFixture::text_type()));
    }

    #[test]
    fn only_the_last_item_meets_the_expectation() {
        let block = BodyFixture::block(vec![BodyFixture::truth(), BodyFixture::truth()]);
        let mut fixture = BodyFixture::default();
        let expected = BodyFixture::text_type();
        let _ = Body::new(&block).type_with(&mut fixture, Some(Expectation::plain(&expected)));
        assert_eq!(fixture.fitted, [expected]);
    }

    #[test]
    fn a_body_that_ends_in_a_let_is_unit() {
        let block = BodyFixture::block(vec![BodyFixture::truth(), BodyFixture::binding(None)]);
        let typed = Body::new(&block)
            .type_with(&mut BodyFixture::default(), None)
            .map(|(_, ty)| ty)
            .ok();
        assert_eq!(typed, Some(Type::unit()));
    }
}
