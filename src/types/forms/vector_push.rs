use super::VectorPushTyping;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::assumed_collections::CollectionAssumption;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct VectorPush<'e> {
    item: &'e Expr,
    span: Span,
}

impl<'e> VectorPush<'e> {
    pub(in crate::types) fn new(item: &'e Expr, span: Span) -> Self {
        Self { item, span }
    }

    pub(in crate::types) fn type_with<C: VectorPushTyping>(
        &self,
        context: &mut C,
        target: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        context.assume_unknown_collection(&target, CollectionAssumption::PushTarget);
        let element = context.fresh();
        let parameters = vec![
            Type::mutable(context.nested_vector_of(1, &element)),
            element,
        ];
        context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::PUSH.text(),
                parameters,
                result: Type::unit(),
            },
            value: target,
            arguments: std::slice::from_ref(self.item),
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::VectorPush;
    use crate::core::Literal;
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::assumed_collections::CollectionAssumption;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{CollectionAssumptions, CollectionShapes, SyntheticCalls, Unknowns};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::synthetic_template::SyntheticTemplate;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct PushFixture {
        assumed: Vec<bool>,
        calls: Vec<(String, Vec<Type>)>,
    }

    impl PushFixture {
        fn push(&mut self) -> Result<TypedExpr, Diagnostic> {
            let item = FormFixtures::leaf(Literal::Bool(true));
            let target = FormFixtures::typed_leaf(Literal::Bool(true), Type::bool());
            VectorPush::new(&item, FormFixtures::span()).type_with(self, target)
        }
    }

    impl Unknowns for PushFixture {
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

    impl CollectionShapes for PushFixture {
        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }

        fn slice_of(&self, element: Type) -> Type {
            CollectionFixtures::slice_of(element)
        }
    }

    impl CollectionAssumptions for PushFixture {
        fn assume_unknown_collection(
            &mut self,
            _target: &TypedExpr,
            assumption: CollectionAssumption,
        ) {
            self.assumed
                .push(matches!(assumption, CollectionAssumption::PushTarget));
        }
    }

    impl SyntheticCalls for PushFixture {
        fn type_synthetic_call(
            &mut self,
            call: SyntheticCall<'_>,
        ) -> Result<TypedExpr, Diagnostic> {
            self.calls.push((
                call.signature.template.to_string(),
                call.signature.parameters,
            ));
            Ok(call.value)
        }

        fn pass_argument(
            &mut self,
            _position: usize,
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
    fn a_push_target_may_be_assumed_a_vector() {
        let mut fixture = PushFixture::default();
        let _ = fixture.push();
        assert_eq!(fixture.assumed, [true]);
    }

    #[test]
    fn a_push_passes_a_mutable_vector_and_its_element() {
        let mut fixture = PushFixture::default();
        let _ = fixture.push();
        let element = CollectionFixtures::element();
        let expected = vec![
            Type::mutable(CollectionFixtures::vector_of(element.clone())),
            element,
        ];
        assert!(matches!(fixture.calls.as_slice(), [(_, parameters)] if *parameters == expected));
    }

    #[test]
    fn a_push_calls_the_push_template() {
        let mut fixture = PushFixture::default();
        let _ = fixture.push();
        let expected = SyntheticTemplate::PUSH.text();
        assert!(matches!(fixture.calls.as_slice(), [(template, _)] if template == expected));
    }
}
