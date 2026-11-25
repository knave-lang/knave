use super::{ExpressionForm, LiteralUnknowns};
use crate::core::{Literal, PrimitiveType};
use crate::diagnostic::Diagnostic;
use crate::reader::NumberValue;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::types::unifier::LiteralKind;

pub(in crate::types) struct LiteralValue<'e> {
    literal: &'e Literal,
    span: Span,
}

impl<'e> LiteralValue<'e> {
    pub(in crate::types) fn new(literal: &'e Literal, span: Span) -> Self {
        Self { literal, span }
    }

    pub(in crate::types) fn literal_type<C: LiteralUnknowns>(&self, context: &mut C) -> Type {
        match self.literal {
            Literal::String(_) => Type::str_ref(),
            Literal::Bool(_) => Type::Primitive(PrimitiveType::Bool),
            Literal::Number(number) => match number.value() {
                NumberValue::Integer(_) => context.fresh_literal(LiteralKind::Integer),
                NumberValue::Float(_) => context.fresh_literal(LiteralKind::Float),
            },
        }
    }
}

impl<C: LiteralUnknowns> ExpressionForm<C> for LiteralValue<'_> {
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        Ok(TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Literal(self.literal.clone())),
            ty: self.literal_type(context),
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::LiteralValue;
    use crate::core::{Literal, PrimitiveType};
    use crate::reader::{Number, NumberValue};
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{ExpressionForm, LiteralUnknowns};
    use crate::types::ty::Type;
    use crate::types::unifier::LiteralKind;

    #[derive(Default)]
    struct LiteralFixture {
        kinds: Vec<LiteralKind>,
    }

    impl LiteralFixture {
        fn type_of(&mut self, literal: Literal) -> Option<Type> {
            LiteralValue::new(&literal, FormFixtures::span())
                .type_against(self, None)
                .ok()
                .map(|typed| typed.ty)
        }

        fn number(value: NumberValue) -> Literal {
            Literal::Number(Number::new("1", value))
        }
    }

    impl LiteralUnknowns for LiteralFixture {
        fn fresh_literal(&mut self, kind: LiteralKind) -> Type {
            self.kinds.push(kind);
            Type::Primitive(PrimitiveType::U8)
        }

        fn is_literal(&self, _value: &Type) -> bool {
            false
        }
    }

    #[test]
    fn a_string_literal_is_a_string_reference() {
        let ty = LiteralFixture::default().type_of(Literal::String(Rc::from("x")));
        assert_eq!(ty, Some(Type::str_ref()));
    }

    #[test]
    fn a_bool_literal_is_a_bool() {
        let ty = LiteralFixture::default().type_of(Literal::Bool(true));
        assert_eq!(ty, Some(Type::bool()));
    }

    #[test]
    fn an_integer_literal_is_a_fresh_integer() {
        let mut fixture = LiteralFixture::default();
        fixture.type_of(LiteralFixture::number(NumberValue::Integer(1)));
        assert!(matches!(fixture.kinds.as_slice(), [LiteralKind::Integer]));
    }

    #[test]
    fn a_float_literal_is_a_fresh_float() {
        let mut fixture = LiteralFixture::default();
        fixture.type_of(LiteralFixture::number(NumberValue::Float(1.5)));
        assert!(matches!(fixture.kinds.as_slice(), [LiteralKind::Float]));
    }
}
