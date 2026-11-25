use super::{ArgumentTyping, ExpressionForm, RecurTargets};
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::typed::{TypedExpr, TypedExprKind};

pub(in crate::types) struct Recurrence<'e> {
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> Recurrence<'e> {
    pub(in crate::types) fn new(arguments: &'e [Expr], span: Span) -> Self {
        Self { arguments, span }
    }
}

impl<C: RecurTargets + ArgumentTyping> ExpressionForm<C> for Recurrence<'_> {
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let target = context.recur_target();
        let arguments = context.type_arguments(None, &target.parameters, self.arguments)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Recur(arguments),
            ty: target.result,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Recurrence;
    use crate::core::{Expr, Literal, PrimitiveType};
    use crate::diagnostic::Diagnostic;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{ArgumentTyping, ExpressionForm, RecurTargets};
    use crate::types::recur_target::RecurTarget;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct RecurFixture {
        typed_against: Vec<Vec<Type>>,
    }

    impl RecurFixture {
        fn parameters() -> Vec<Type> {
            vec![Type::Primitive(PrimitiveType::I64)]
        }

        fn result() -> Type {
            Type::Primitive(PrimitiveType::U8)
        }

        fn recur(&mut self) -> Result<TypedExpr, Diagnostic> {
            let arguments = [FormFixtures::leaf(Literal::Bool(true))];
            Recurrence::new(&arguments, FormFixtures::span()).type_against(self, None)
        }
    }

    impl RecurTargets for RecurFixture {
        fn recur_target(&self) -> RecurTarget {
            RecurTarget {
                parameters: Self::parameters(),
                result: Self::result(),
            }
        }
    }

    impl ArgumentTyping for RecurFixture {
        fn type_arguments(
            &mut self,
            _callee: Option<&TypedExpr>,
            parameters: &[Type],
            _arguments: &[Expr],
        ) -> Result<Vec<TypedArgument>, Diagnostic> {
            self.typed_against.push(parameters.to_vec());
            Ok(Vec::new())
        }

        fn pass_first_argument(
            &mut self,
            _callee: &TypedExpr,
            _parameter: &Type,
            _value: &TypedExpr,
        ) -> Result<Borrow, Diagnostic> {
            Ok(Borrow::None)
        }
    }

    #[test]
    fn a_recur_has_the_function_result_type() {
        let ty = RecurFixture::default().recur().ok().map(|typed| typed.ty);
        assert_eq!(ty, Some(RecurFixture::result()));
    }

    #[test]
    fn a_recur_types_its_arguments_against_the_function_parameters() {
        let mut fixture = RecurFixture::default();
        let _ = fixture.recur();
        assert_eq!(fixture.typed_against, [RecurFixture::parameters()]);
    }
}
