use super::if_operands::IfOperands;
use super::{ChildTyping, ExpressionForm, MacroOrigins, TypeFitting};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::problems::MismatchContext;
use crate::types::result_span::ResultSpan;
use crate::types::ty::Type;
use crate::types::typed::{TypedControl, TypedExpr, TypedExprKind};

pub(in crate::types) struct Conditional<'e> {
    operands: IfOperands<'e>,
    span: Span,
}

impl<'e> Conditional<'e> {
    pub(in crate::types) fn new(operands: IfOperands<'e>, span: Span) -> Self {
        Self { operands, span }
    }
}

impl<C> ExpressionForm<C> for Conditional<'_>
where
    C: ChildTyping + TypeFitting + MacroOrigins,
{
    fn type_against(
        &self,
        context: &mut C,
        _expectation: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let test = context.type_child(self.operands.test)?;
        let expanded_from = context.macro_name(self.span);
        let bool_type = Type::bool();
        let test_context = MismatchContext::IfTest {
            expanded_from: expanded_from.clone(),
        };
        context.fit(
            Expectation::new(&bool_type, test_context),
            &test.ty,
            test.span,
        )?;
        let then = context.type_child(self.operands.then)?;
        let otherwise = context.type_child(self.operands.otherwise)?;
        let branch_context = MismatchContext::IfBranch {
            first: ResultSpan::of(&then),
            expanded_from,
        };
        context.fit(
            Expectation::new(&then.ty, branch_context),
            &otherwise.ty,
            ResultSpan::of(&otherwise),
        )?;
        Ok(TypedExpr {
            ty: then.ty.clone(),
            kind: TypedExprKind::Control(TypedControl::If(
                Box::new(test),
                Box::new(then),
                Box::new(otherwise),
            )),
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Conditional;
    use crate::core::{Expr, ExprKind, Leaf, Literal, PrimitiveType};
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::expectation::Expectation;
    use crate::types::forms::IfOperands;
    use crate::types::forms::{ChildTyping, ExpressionForm, MacroOrigins, TypeFitting};
    use crate::types::problems::MismatchContext;
    use crate::types::ty::Type;
    use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};

    struct Fit {
        expected: Type,
        found: Type,
        span: Span,
        expanded_from: Option<Rc<str>>,
    }

    struct ConditionalFixture {
        macro_name: Option<Rc<str>>,
        fits: Vec<Fit>,
    }

    impl ConditionalFixture {
        const MACRO: &str = "unless";

        fn new() -> Self {
            Self {
                macro_name: Some(Rc::from(Self::MACRO)),
                fits: Vec::new(),
            }
        }

        fn type_if(&mut self, test: &Expr, then: &Expr, otherwise: &Expr) -> TypedExpr {
            let operands = IfOperands {
                test,
                then,
                otherwise,
            };
            Conditional::new(operands, Self::span(0))
                .type_against(self, None)
                .unwrap_or_else(|_| panic!("the fixture fits every type"))
        }

        fn boolean(at: usize) -> Expr {
            Self::literal(Literal::Bool(true), at)
        }

        fn text(at: usize) -> Expr {
            Self::literal(Literal::String(Rc::from("x")), at)
        }

        fn literal(literal: Literal, at: usize) -> Expr {
            Expr::new(ExprKind::Leaf(Leaf::Literal(literal)), Self::span(at))
        }

        fn span(at: usize) -> Span {
            Span::new(SourceId::new(0), at..at + 1)
        }

        fn text_type() -> Type {
            Type::Primitive(PrimitiveType::Str)
        }
    }

    impl ChildTyping for ConditionalFixture {
        fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
            let ExprKind::Leaf(Leaf::Literal(literal)) = child.kind() else {
                panic!("the fixture types literals only")
            };
            let ty = match literal {
                Literal::Bool(_) => Type::bool(),
                _ => Self::text_type(),
            };
            Ok(TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(literal.clone())),
                ty,
                span: child.span(),
            })
        }
    }

    impl TypeFitting for ConditionalFixture {
        fn fit(
            &mut self,
            expectation: Expectation,
            found: &Type,
            span: Span,
        ) -> Result<(), Diagnostic> {
            let expanded_from = match expectation.context {
                MismatchContext::IfTest { expanded_from }
                | MismatchContext::IfBranch { expanded_from, .. } => expanded_from,
                _ => None,
            };
            self.fits.push(Fit {
                expected: expectation.expected.clone(),
                found: found.clone(),
                span,
                expanded_from,
            });
            Ok(())
        }
    }

    impl MacroOrigins for ConditionalFixture {
        fn macro_name(&self, _span: Span) -> Option<Rc<str>> {
            self.macro_name.clone()
        }
    }

    #[test]
    fn the_test_is_fitted_to_bool() {
        let mut fixture = ConditionalFixture::new();
        let test = ConditionalFixture::text(1);
        fixture.type_if(
            &test,
            &ConditionalFixture::boolean(2),
            &ConditionalFixture::boolean(3),
        );
        let fit = &fixture.fits[0];
        assert_eq!(
            (fit.expected.clone(), fit.found.clone(), fit.span),
            (
                Type::bool(),
                ConditionalFixture::text_type(),
                ConditionalFixture::span(1)
            )
        );
    }

    #[test]
    fn the_other_branch_is_fitted_to_the_first() {
        let mut fixture = ConditionalFixture::new();
        let then = ConditionalFixture::boolean(2);
        let otherwise = ConditionalFixture::text(3);
        fixture.type_if(&ConditionalFixture::boolean(1), &then, &otherwise);
        let fit = &fixture.fits[1];
        assert_eq!(
            (fit.expected.clone(), fit.found.clone(), fit.span),
            (
                Type::bool(),
                ConditionalFixture::text_type(),
                ConditionalFixture::span(3)
            )
        );
    }

    #[test]
    fn both_fits_name_the_macro_the_if_came_from() {
        let mut fixture = ConditionalFixture::new();
        fixture.type_if(
            &ConditionalFixture::boolean(1),
            &ConditionalFixture::boolean(2),
            &ConditionalFixture::boolean(3),
        );
        let origins: Vec<Option<Rc<str>>> = fixture
            .fits
            .iter()
            .map(|fit| fit.expanded_from.clone())
            .collect();
        let expected = Some(Rc::from(ConditionalFixture::MACRO));
        assert_eq!(origins, vec![expected.clone(), expected]);
    }

    #[test]
    fn the_if_takes_the_type_of_its_first_branch() {
        let mut fixture = ConditionalFixture::new();
        let typed = fixture.type_if(
            &ConditionalFixture::boolean(1),
            &ConditionalFixture::boolean(2),
            &ConditionalFixture::text(3),
        );
        assert_eq!(typed.ty, Type::bool());
    }
}
