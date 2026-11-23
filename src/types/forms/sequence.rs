use super::{
    AnnotationLowering, Body, ChildTyping, ExpectedChildTyping, ExpressionForm, LocalScope,
};
use crate::core::Block;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::typed::{TypedControl, TypedExpr, TypedExprKind};

pub(in crate::types) struct Sequence<'e> {
    body: Body<'e>,
    span: Span,
}

impl<'e> Sequence<'e> {
    pub(in crate::types) fn new(block: &'e Block, span: Span) -> Self {
        Self {
            body: Body::new(block),
            span,
        }
    }
}

impl<C> ExpressionForm<C> for Sequence<'_>
where
    C: ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope,
{
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let (block, ty) = self.body.type_with(context, None)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Control(TypedControl::Block(block)),
            ty,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Sequence;
    use crate::types::forms::ExpressionForm;
    use crate::types::forms::body_fixture::BodyFixture;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::typed::{TypedControl, TypedExprKind};

    #[test]
    fn a_do_form_is_a_block_at_its_span() {
        let block = BodyFixture::block(vec![BodyFixture::truth()]);
        let typed = Sequence::new(&block, FormFixtures::span())
            .type_against(&mut BodyFixture::default(), None)
            .ok();
        assert!(matches!(
            typed,
            Some(expr) if matches!(expr.kind, TypedExprKind::Control(TypedControl::Block(_)))
                && expr.span == FormFixtures::span()
        ));
    }
}
