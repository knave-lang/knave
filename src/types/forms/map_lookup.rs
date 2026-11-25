use super::MapLookupTyping;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind};

pub(in crate::types) struct MapLookup<'e> {
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> MapLookup<'e> {
    const QUERY_POSITION: usize = 1;

    pub(in crate::types) fn new(arguments: &'e [Expr], span: Span) -> Self {
        Self { arguments, span }
    }

    pub(in crate::types) fn type_with<C: MapLookupTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
        map: Collection,
    ) -> Result<TypedExpr, Diagnostic> {
        context.require_map_key(&map, value.span);
        let [query] = self.arguments else {
            let found = self.arguments.len();
            return Err(TypeProblems::wrong_argument_count(1, found, self.span));
        };
        let query_type = context.fresh();
        let parameters = vec![Type::shared(map.ty().clone()), Type::shared(query_type)];
        let result = context.option_of(map.value().clone());
        let call = context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::LOOK_UP.text(),
                parameters,
                result,
            },
            value,
            arguments: self.arguments,
            span: self.span,
        })?;
        if let Some(passed_query) = Self::passed_query(&call) {
            context.require_key_borrow(&map, &passed_query, query.span());
        }
        Ok(call)
    }

    fn passed_query(call: &TypedExpr) -> Option<Type> {
        match &call.kind {
            TypedExprKind::Call(_, arguments) => arguments
                .get(Self::QUERY_POSITION)
                .map(|argument| argument.value.ty.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MapLookup;
    use crate::core::{Expr, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::collection::Collection;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{MapKeys, PreludeShapes, SyntheticCalls, Unknowns};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind};

    #[derive(Default)]
    struct LookupFixture {
        required_keys: usize,
        borrows_checked: Vec<Type>,
        calls: Vec<(Vec<Type>, Type)>,
    }

    impl LookupFixture {
        fn get(&mut self, arguments: usize) -> Result<TypedExpr, Diagnostic> {
            let arguments: Vec<Expr> = (0..arguments).map(|_| Self::truth()).collect();
            let map = CollectionFixtures::new().collection(&CollectionFixtures::map());
            let value = FormFixtures::typed_leaf(Literal::Bool(true), CollectionFixtures::map());
            MapLookup::new(&arguments, FormFixtures::span()).type_with(self, value, map)
        }

        fn truth() -> Expr {
            FormFixtures::leaf(Literal::Bool(true))
        }

        fn option_of(value: Type) -> Type {
            CollectionFixtures::vector_of(value)
        }

        fn passed_argument(ty: Type) -> TypedArgument {
            TypedArgument {
                value: FormFixtures::typed_leaf(Literal::Bool(true), ty),
                borrow: Borrow::None,
            }
        }
    }

    impl MapKeys for LookupFixture {
        fn require_map_key(&mut self, _map: &Collection, _span: Span) {
            self.required_keys += 1;
        }

        fn require_key_borrow(&mut self, _map: &Collection, query: &Type, _span: Span) {
            self.borrows_checked.push(query.clone());
        }
    }

    impl Unknowns for LookupFixture {
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

    impl PreludeShapes for LookupFixture {
        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }

        fn slice_of(&self, element: Type) -> Type {
            CollectionFixtures::slice_of(element)
        }

        fn option_of(&self, value: Type) -> Type {
            Self::option_of(value)
        }

        fn iterator_of(&self, _item: Type) -> Type {
            unreachable!("this form builds no iterator")
        }
    }

    impl SyntheticCalls for LookupFixture {
        fn type_synthetic_call(
            &mut self,
            call: SyntheticCall<'_>,
        ) -> Result<TypedExpr, Diagnostic> {
            let arguments = vec![
                Self::passed_argument(CollectionFixtures::map()),
                Self::passed_argument(CollectionFixtures::key()),
            ];
            let callee = FormFixtures::typed_leaf(Literal::Bool(true), Type::unit());
            let typed = TypedExpr {
                kind: TypedExprKind::Call(Box::new(callee), arguments),
                ty: call.signature.result.clone(),
                span: call.span,
            };
            self.calls
                .push((call.signature.parameters, call.signature.result));
            Ok(typed)
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
    fn a_map_lookup_requires_its_key_bounds() {
        let mut fixture = LookupFixture::default();
        let _ = fixture.get(1);
        assert_eq!(fixture.required_keys, 1);
    }

    #[test]
    fn a_map_lookup_without_one_query_is_refused() {
        let mut fixture = LookupFixture::default();
        assert!(fixture.get(2).is_err());
        assert!(fixture.calls.is_empty());
    }

    #[test]
    fn a_map_lookup_gives_an_option_of_the_map_value() {
        let mut fixture = LookupFixture::default();
        let _ = fixture.get(1);
        let expected = LookupFixture::option_of(Type::bool());
        assert!(matches!(fixture.calls.as_slice(), [(_, result)] if *result == expected));
    }

    #[test]
    fn a_map_lookup_borrows_the_map_and_the_query() {
        let mut fixture = LookupFixture::default();
        let _ = fixture.get(1);
        let expected = vec![
            Type::shared(CollectionFixtures::map()),
            Type::shared(CollectionFixtures::element()),
        ];
        assert!(matches!(fixture.calls.as_slice(), [(parameters, _)] if *parameters == expected));
    }

    #[test]
    fn a_map_lookup_checks_the_borrow_of_the_passed_query() {
        let mut fixture = LookupFixture::default();
        let _ = fixture.get(1);
        assert_eq!(fixture.borrows_checked, [CollectionFixtures::key()]);
    }
}
