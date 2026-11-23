use super::{Body, BodyTyping, ExpressionForm};
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
    C: BodyTyping,
{
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        self.body
            .type_with(context, None)
            .map(|(block, ty)| TypedExpr {
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

    const NESTED_DO_FORM_STACK_BUDGET: usize = 4 * 1024;

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

    #[test]
    fn a_nested_do_form_stays_within_its_stack_budget() {
        let block = BodyFixture::nested_do_forms(2);
        let mut fixture = BodyFixture::default();
        let _ = Sequence::new(&block, FormFixtures::span()).type_against(&mut fixture, None);
        let [outer, inner, ..] = fixture.stack_marks[..] else {
            panic!("each nested do form types its child")
        };
        assert!(outer - inner <= NESTED_DO_FORM_STACK_BUDGET);
    }
}
