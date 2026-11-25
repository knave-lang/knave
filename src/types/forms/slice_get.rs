use super::SliceGetTyping;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::assumed_collections::CollectionAssumption;
use crate::types::collection::Collection;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct SliceGet<'e> {
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> SliceGet<'e> {
    pub(in crate::types) fn new(arguments: &'e [Expr], span: Span) -> Self {
        Self { arguments, span }
    }

    pub(in crate::types) fn type_with<C: SliceGetTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let [_index] = self.arguments else {
            let found = self.arguments.len();
            return Err(TypeProblems::wrong_argument_count(1, found, self.span));
        };
        context.assume_unknown_collection(&value, CollectionAssumption::GetCollection);
        let element = context.fresh();
        let parameters = vec![
            Type::shared(context.slice_of(element.clone())),
            Collection::INDEX,
        ];
        let result = context.option_of(element);
        context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::SLICE_LOOK_UP.text(),
                parameters,
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
    use super::SliceGet;
    use crate::core::{Expr, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::assumed_collections::CollectionAssumption;
    use crate::types::collection::Collection;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{CollectionAssumptions, PreludeShapes, SyntheticCalls, Unknowns};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct SliceFixture {
        assumptions: Vec<CollectionAssumption>,
        calls: Vec<(Vec<Type>, Type)>,
    }

    impl SliceFixture {
        fn get(&mut self, arguments: usize) -> Result<TypedExpr, Diagnostic> {
            let arguments: Vec<Expr> = (0..arguments)
                .map(|_| FormFixtures::leaf(Literal::Bool(true)))
                .collect();
            let value = FormFixtures::typed_leaf(Literal::Bool(true), Type::bool());
            SliceGet::new(&arguments, FormFixtures::span()).type_with(self, value)
        }

        fn option_of(value: Type) -> Type {
            CollectionFixtures::vector_of(value)
        }
    }

    impl Unknowns for SliceFixture {
        fn fresh(&mut self) -> Type {
            CollectionFixtures::element()
        }

        fn resolve(&self, value: &Type) -> Type {
            value.clone()
        }

        fn is_open(&self, _value: &Type) -> bool {
            false
        }
    }

    impl PreludeShapes for SliceFixture {
        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }

        fn slice_of(&self, element: Type) -> Type {
            CollectionFixtures::slice_of(element)
        }

        fn option_of(&self, value: Type) -> Type {
            Self::option_of(value)
        }
    }

    impl CollectionAssumptions for SliceFixture {
        fn assume_unknown_collection(
            &mut self,
            _target: &TypedExpr,
            assumption: CollectionAssumption,
        ) {
            self.assumptions.push(assumption);
        }
    }

    impl SyntheticCalls for SliceFixture {
        fn type_synthetic_call(
            &mut self,
            call: SyntheticCall<'_>,
        ) -> Result<TypedExpr, Diagnostic> {
            self.calls
                .push((call.signature.parameters, call.signature.result));
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
    fn a_get_without_one_index_is_refused() {
        let mut fixture = SliceFixture::default();
        assert!(fixture.get(2).is_err());
        assert!(fixture.calls.is_empty());
    }

    #[test]
    fn a_get_assumes_its_target_a_collection() {
        let mut fixture = SliceFixture::default();
        let _ = fixture.get(1);
        assert!(matches!(
            fixture.assumptions.as_slice(),
            [CollectionAssumption::GetCollection]
        ));
    }

    #[test]
    fn a_get_borrows_a_slice_and_takes_an_index() {
        let mut fixture = SliceFixture::default();
        let _ = fixture.get(1);
        let slice = CollectionFixtures::slice_of(CollectionFixtures::element());
        let expected = vec![Type::shared(slice), Collection::INDEX];
        assert!(matches!(fixture.calls.as_slice(), [(parameters, _)] if *parameters == expected));
    }

    #[test]
    fn a_get_gives_an_option_of_the_element() {
        let mut fixture = SliceFixture::default();
        let _ = fixture.get(1);
        let expected = SliceFixture::option_of(CollectionFixtures::element());
        assert!(matches!(fixture.calls.as_slice(), [(_, result)] if *result == expected));
    }
}
