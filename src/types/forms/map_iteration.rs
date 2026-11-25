use super::MapIterationTyping;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

enum MapSide {
    Keys,
    Values,
}

pub(in crate::types) struct MapIteration<'e> {
    side: MapSide,
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> MapIteration<'e> {
    pub(in crate::types) fn keys(arguments: &'e [Expr], span: Span) -> Self {
        let side = MapSide::Keys;
        Self {
            side,
            arguments,
            span,
        }
    }

    pub(in crate::types) fn values(arguments: &'e [Expr], span: Span) -> Self {
        let side = MapSide::Values;
        Self {
            side,
            arguments,
            span,
        }
    }

    pub(in crate::types) fn type_with<C: MapIterationTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
        map: Collection,
    ) -> Result<TypedExpr, Diagnostic> {
        if !self.arguments.is_empty() {
            let found = self.arguments.len();
            return Err(TypeProblems::wrong_argument_count(0, found, self.span));
        }
        let (item, template) = match self.side {
            MapSide::Keys => (map.key().clone(), SyntheticTemplate::KEYS.text()),
            MapSide::Values => (map.value().clone(), SyntheticTemplate::VALUES.text()),
        };
        let result = context.iterator_of(Type::shared(item));
        context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template,
                parameters: vec![Type::shared(map.ty().clone())],
                result,
            },
            value,
            arguments: self.arguments,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MapIteration;
    use crate::core::{Expr, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{PreludeShapes, SyntheticCalls};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::synthetic_template::SyntheticTemplate;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct MapSideFixture {
        calls: Vec<(String, Vec<Type>, Type)>,
    }

    impl MapSideFixture {
        fn iterate(&mut self, iteration: MapIteration<'_>) -> Result<TypedExpr, Diagnostic> {
            let map = CollectionFixtures::new().collection(&CollectionFixtures::map());
            let value = FormFixtures::typed_leaf(Literal::Bool(true), CollectionFixtures::map());
            iteration.type_with(self, value, map)
        }

        fn iterator_of(item: Type) -> Type {
            CollectionFixtures::vector_of(item)
        }

        fn result(&self) -> Option<&Type> {
            match self.calls.as_slice() {
                [(_, _, result)] => Some(result),
                _ => None,
            }
        }
    }

    impl PreludeShapes for MapSideFixture {
        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }

        fn slice_of(&self, element: Type) -> Type {
            CollectionFixtures::slice_of(element)
        }

        fn option_of(&self, _value: Type) -> Type {
            unreachable!("this form builds no option")
        }

        fn iterator_of(&self, item: Type) -> Type {
            Self::iterator_of(item)
        }
    }

    impl SyntheticCalls for MapSideFixture {
        fn type_synthetic_call(
            &mut self,
            call: SyntheticCall<'_>,
        ) -> Result<TypedExpr, Diagnostic> {
            let SyntheticSignature {
                template,
                parameters,
                result,
            } = call.signature;
            self.calls.push((template.to_string(), parameters, result));
            Ok(call.value)
        }

        fn pass_argument(
            &mut self,
            _parameter: &Type,
            _value: &TypedExpr,
        ) -> Result<Borrow, Diagnostic> {
            Ok(Borrow::None)
        }

        fn finish_call(
            &mut self,
            signature: SyntheticSignature<'_>,
            _arguments: Vec<TypedArgument>,
            _span: Span,
        ) -> TypedExpr {
            FormFixtures::typed_leaf(Literal::Bool(true), signature.result)
        }
    }

    #[test]
    fn a_map_iteration_with_an_argument_is_refused() {
        let arguments = [FormFixtures::leaf(Literal::Bool(true))];
        let mut fixture = MapSideFixture::default();
        let iteration = MapIteration::keys(&arguments, FormFixtures::span());
        assert!(fixture.iterate(iteration).is_err());
        assert!(fixture.calls.is_empty());
    }

    #[test]
    fn keys_yield_borrowed_keys_through_the_keys_template() {
        let arguments: [Expr; 0] = [];
        let mut fixture = MapSideFixture::default();
        let _ = fixture.iterate(MapIteration::keys(&arguments, FormFixtures::span()));
        let expected = (
            SyntheticTemplate::KEYS.text().to_string(),
            vec![Type::shared(CollectionFixtures::map())],
            MapSideFixture::iterator_of(Type::shared(CollectionFixtures::key())),
        );
        assert_eq!(fixture.calls, [expected]);
    }

    #[test]
    fn values_yield_borrowed_values() {
        let arguments: [Expr; 0] = [];
        let mut fixture = MapSideFixture::default();
        let _ = fixture.iterate(MapIteration::values(&arguments, FormFixtures::span()));
        let expected = MapSideFixture::iterator_of(Type::shared(Type::bool()));
        assert_eq!(fixture.result(), Some(&expected));
    }
}
