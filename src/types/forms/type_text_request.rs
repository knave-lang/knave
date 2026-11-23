use super::{ExpressionForm, OperandTyping, TypeTextRecords};
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind};

pub(in crate::types) struct TypeTextRequest<'e> {
    operand: &'e Expr,
    span: Span,
}

impl<'e> TypeTextRequest<'e> {
    pub(in crate::types) fn new(operand: &'e Expr, span: Span) -> Self {
        Self { operand, span }
    }
}

impl<C> ExpressionForm<C> for TypeTextRequest<'_>
where
    C: OperandTyping + TypeTextRecords,
{
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        context.note_type_text();
        let operand = context.type_operand(self.operand)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Infer(Box::new(operand)),
            ty: Type::str_ref(),
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TypeTextRequest;
    use crate::core::{Expr, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{ExpressionForm, OperandTyping, TypeTextRecords};
    use crate::types::ty::Type;
    use crate::types::typed::TypedExpr;

    #[derive(Default)]
    struct TypeTextFixture {
        operands: usize,
        notes: usize,
    }

    impl TypeTextFixture {
        fn request(&mut self) -> TypedExpr {
            let operand = FormFixtures::leaf(Literal::Bool(true));
            TypeTextRequest::new(&operand, FormFixtures::span())
                .type_against(self, None)
                .unwrap_or_else(|_| panic!("the fixture types every operand"))
        }
    }

    impl OperandTyping for TypeTextFixture {
        fn type_operand(&mut self, _operand: &Expr) -> Result<TypedExpr, Diagnostic> {
            self.operands += 1;
            Ok(FormFixtures::typed_leaf(Literal::Bool(true), Type::bool()))
        }
    }

    impl TypeTextRecords for TypeTextFixture {
        fn note_type_text(&mut self) {
            self.notes += 1;
        }
    }

    #[test]
    fn a_type_text_is_a_borrowed_str() {
        assert_eq!(TypeTextFixture::default().request().ty, Type::str_ref());
    }

    #[test]
    fn a_type_text_is_noted_and_its_operand_typed_as_an_operand() {
        let mut fixture = TypeTextFixture::default();
        fixture.request();
        assert_eq!((fixture.notes, fixture.operands), (1, 1));
    }
}
