use super::SliceIterationTyping;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct SliceIteration {
    span: Span,
}

impl SliceIteration {
    pub(in crate::types) fn new(span: Span) -> Self {
        Self { span }
    }

    pub(in crate::types) fn type_with<C: SliceIterationTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let element = context.fresh();
        let parameter = Type::shared(context.slice_of(element.clone()));
        let result = context.iterator_of(Type::shared(element));
        context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::ITERATE.text(),
                parameters: vec![parameter],
                result,
            },
            value,
            arguments: &[],
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SliceIteration;
    use crate::core::Literal;
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{PreludeShapes, SyntheticCalls, Unknowns};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct SliceFixture {
        calls: Vec<(Vec<Type>, Type)>,
    }

    impl SliceFixture {
        fn iterate(&mut self) -> Result<TypedExpr, Diagnostic> {
            let value = FormFixtures::typed_leaf(Literal::Bool(true), Type::bool());
            SliceIteration::new(FormFixtures::span()).type_with(self, value)
        }

        fn iterator_of(item: Type) -> Type {
            CollectionFixtures::vector_of(item)
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

        fn option_of(&self, _value: Type) -> Type {
            unreachable!("this form builds no option")
        }

        fn iterator_of(&self, item: Type) -> Type {
            Self::iterator_of(item)
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
    fn a_slice_iteration_borrows_a_slice_of_a_fresh_element() {
        let mut fixture = SliceFixture::default();
        let _ = fixture.iterate();
        let slice = CollectionFixtures::slice_of(CollectionFixtures::element());
        let expected = vec![Type::shared(slice)];
        assert!(matches!(fixture.calls.as_slice(), [(parameters, _)] if *parameters == expected));
    }

    #[test]
    fn a_slice_iteration_yields_borrowed_elements() {
        let mut fixture = SliceFixture::default();
        let _ = fixture.iterate();
        let expected = SliceFixture::iterator_of(Type::shared(CollectionFixtures::element()));
        assert!(matches!(fixture.calls.as_slice(), [(_, result)] if *result == expected));
    }
}
