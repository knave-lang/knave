use super::{CollectionIterationTyping, SliceIteration};
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::collection::CollectionKind;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct CollectionIteration<'e> {
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> CollectionIteration<'e> {
    pub(in crate::types) fn new(arguments: &'e [Expr], span: Span) -> Self {
        Self { arguments, span }
    }

    pub(in crate::types) fn type_with<C: CollectionIterationTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        if !self.arguments.is_empty() {
            let found = self.arguments.len();
            return Err(TypeProblems::wrong_argument_count(0, found, self.span));
        }
        let Some((item, collection_type)) = context.collection_item(&value.ty) else {
            return self.iterate_unknown(context, value);
        };
        if let Some(collection) = context.collection_of(&collection_type) {
            if collection.kind() == CollectionKind::Vector {
                return SliceIteration::new(self.span).type_with(context, value);
            }
            if collection.is_map() {
                context.require_map_key(&collection, value.span);
            }
        }
        let result = context.iterator_of(item);
        context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::ITERATE.text(),
                parameters: vec![Type::shared(collection_type)],
                result,
            },
            value,
            arguments: self.arguments,
            span: self.span,
        })
    }

    fn iterate_unknown<C: CollectionIterationTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let resolved = context.resolve(&value.ty);
        if resolved.is_unknown() && !context.is_literal(&resolved) {
            return SliceIteration::new(self.span).type_with(context, value);
        }
        Err(TypeProblems::iter_needs_collection(
            &context.describe(&resolved),
            value.span,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::CollectionIteration;
    use crate::core::{Expr, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::collection::Collection;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{
        CollectionLookups, LiteralUnknowns, MapKeys, PreludeShapes, SyntheticCalls, TypeDisplays,
        Unknowns,
    };
    use crate::types::problems::TypeProblems;
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::ty::{Type, TypeVar};
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};
    use crate::types::unifier::LiteralKind;

    #[derive(Default)]
    struct IterationFixture {
        collections: CollectionFixtures,
        item: Option<(Type, Type)>,
        literal: bool,
        required_keys: usize,
        calls: Vec<(Vec<Type>, Type)>,
    }

    impl IterationFixture {
        fn of(collection: Type) -> Self {
            Self {
                item: Some((Self::item(), collection)),
                ..Default::default()
            }
        }

        fn iterate(&mut self, arguments: usize) -> Result<TypedExpr, Diagnostic> {
            let arguments: Vec<Expr> = (0..arguments)
                .map(|_| FormFixtures::leaf(Literal::Bool(true)))
                .collect();
            let value = FormFixtures::typed_leaf(Literal::Bool(true), Type::Var(TypeVar::new(0)));
            CollectionIteration::new(&arguments, FormFixtures::span()).type_with(self, value)
        }

        fn item() -> Type {
            Type::shared(Type::bool())
        }

        fn iterator_of(item: Type) -> Type {
            CollectionFixtures::vector_of(item)
        }

        fn borrowed_slice() -> Vec<Type> {
            vec![Type::shared(CollectionFixtures::slice_of(
                CollectionFixtures::element(),
            ))]
        }

        fn parameters(&self) -> Option<&Vec<Type>> {
            match self.calls.as_slice() {
                [(parameters, _)] => Some(parameters),
                _ => None,
            }
        }
    }

    impl Unknowns for IterationFixture {
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

    impl LiteralUnknowns for IterationFixture {
        fn fresh_literal(&mut self, _kind: LiteralKind) -> Type {
            unreachable!("an iteration makes no literal")
        }

        fn is_literal(&self, _value: &Type) -> bool {
            self.literal
        }
    }

    impl CollectionLookups for IterationFixture {
        fn collection_of(&self, ty: &Type) -> Option<Collection> {
            self.collections.collection_of(ty)
        }

        fn collection_of_referent(&self, ty: &Type) -> Option<Collection> {
            self.collections.collection_of_referent(ty)
        }

        fn collection_item(&self, _ty: &Type) -> Option<(Type, Type)> {
            self.item.clone()
        }
    }

    impl PreludeShapes for IterationFixture {
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

    impl MapKeys for IterationFixture {
        fn require_map_key(&mut self, _map: &Collection, _span: Span) {
            self.required_keys += 1;
        }

        fn require_key_borrow(&mut self, _map: &Collection, _query: &Type, _span: Span) {}
    }

    impl TypeDisplays for IterationFixture {
        fn show(&self, _ty: &Type) -> String {
            "shown".to_string()
        }

        fn describe(&mut self, _ty: &Type) -> String {
            "described".to_string()
        }
    }

    impl SyntheticCalls for IterationFixture {
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
    fn an_iteration_with_an_argument_is_refused() {
        let mut fixture = IterationFixture::of(CollectionFixtures::map());
        assert!(fixture.iterate(1).is_err());
        assert!(fixture.calls.is_empty());
    }

    #[test]
    fn an_unknown_base_iterates_as_a_slice() {
        let mut fixture = IterationFixture::default();
        let _ = fixture.iterate(0);
        assert_eq!(
            fixture.parameters(),
            Some(&IterationFixture::borrowed_slice())
        );
    }

    #[test]
    fn a_literal_base_is_refused_as_no_collection() {
        let mut fixture = IterationFixture {
            literal: true,
            ..Default::default()
        };
        let refusal = fixture.iterate(0).err().map(|error| format!("{error:?}"));
        let expected = TypeProblems::iter_needs_collection("described", FormFixtures::span());
        assert_eq!(refusal, Some(format!("{expected:?}")));
    }

    #[test]
    fn a_vector_iterates_as_a_slice() {
        let vector = CollectionFixtures::vector_of(Type::bool());
        let mut fixture = IterationFixture::of(vector);
        let _ = fixture.iterate(0);
        assert_eq!(
            fixture.parameters(),
            Some(&IterationFixture::borrowed_slice())
        );
    }

    #[test]
    fn a_map_iteration_requires_its_key_bounds() {
        let mut fixture = IterationFixture::of(CollectionFixtures::map());
        let _ = fixture.iterate(0);
        assert_eq!(fixture.required_keys, 1);
    }

    #[test]
    fn a_map_iteration_borrows_the_map() {
        let mut fixture = IterationFixture::of(CollectionFixtures::map());
        let _ = fixture.iterate(0);
        let expected = vec![Type::shared(CollectionFixtures::map())];
        assert_eq!(fixture.parameters(), Some(&expected));
    }

    #[test]
    fn a_map_iteration_yields_the_collection_item() {
        let mut fixture = IterationFixture::of(CollectionFixtures::map());
        let _ = fixture.iterate(0);
        let expected = IterationFixture::iterator_of(IterationFixture::item());
        assert!(matches!(fixture.calls.as_slice(), [(_, result)] if *result == expected));
    }
}
