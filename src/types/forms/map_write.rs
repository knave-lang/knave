use super::MapWriteTyping;
use crate::core::KeyedPath;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct MapWrite<'e> {
    path: &'e KeyedPath,
    span: Span,
}

impl<'e> MapWrite<'e> {
    pub(in crate::types) fn new(path: &'e KeyedPath, span: Span) -> Self {
        Self { path, span }
    }

    pub(in crate::types) fn type_with<C: MapWriteTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
        map: Collection,
    ) -> Result<TypedExpr, Diagnostic> {
        context.require_map_key(&map, value.span);
        let [key] = self.path.keys.as_slice() else {
            let found = self.path.keys.len();
            return Err(TypeProblems::wrong_argument_count(1, found, self.span));
        };
        let query_type = context.fresh();
        let parameters = vec![
            Type::mutable(map.ty().clone()),
            Type::shared(query_type.clone()),
            map.value().clone(),
        ];
        let arguments = self.path.keys_and_item();
        let call = context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::SET_AT_KEY.text(),
                parameters,
                result: Type::unit(),
            },
            value,
            arguments: &arguments,
            span: self.span,
        })?;
        let query = context.resolve(&query_type);
        context.require_key_borrow(&map, &query, key.span());
        Ok(call)
    }
}

#[cfg(test)]
mod tests {
    use super::MapWrite;
    use crate::core::{Expr, KeyedPath, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::collection::Collection;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{MapKeys, SyntheticCalls, Unknowns};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct MapFixture {
        required_keys: usize,
        parameters: Vec<Vec<Type>>,
        borrows_checked: Vec<Type>,
    }

    impl MapFixture {
        fn write(&mut self, keys: usize) -> Result<TypedExpr, Diagnostic> {
            let path = KeyedPath {
                value: Box::new(Self::truth()),
                keys: (0..keys).map(|_| Self::truth()).collect(),
                item: Box::new(Self::truth()),
            };
            let map = CollectionFixtures::new().collection(&CollectionFixtures::map());
            let value = FormFixtures::typed_leaf(Literal::Bool(true), CollectionFixtures::map());
            MapWrite::new(&path, FormFixtures::span()).type_with(self, value, map)
        }

        fn truth() -> Expr {
            FormFixtures::leaf(Literal::Bool(true))
        }
    }

    impl MapKeys for MapFixture {
        fn require_map_key(&mut self, _map: &Collection, _span: Span) {
            self.required_keys += 1;
        }

        fn require_key_borrow(&mut self, _map: &Collection, query: &Type, _span: Span) {
            self.borrows_checked.push(query.clone());
        }
    }

    impl Unknowns for MapFixture {
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

    impl SyntheticCalls for MapFixture {
        fn type_synthetic_call(
            &mut self,
            call: SyntheticCall<'_>,
        ) -> Result<TypedExpr, Diagnostic> {
            self.parameters.push(call.signature.parameters);
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
    fn a_map_write_requires_its_key_bounds() {
        let mut fixture = MapFixture::default();
        let _ = fixture.write(1);
        assert_eq!(fixture.required_keys, 1);
    }

    #[test]
    fn a_map_write_takes_one_key() {
        assert!(MapFixture::default().write(2).is_err());
    }

    #[test]
    fn a_map_write_passes_the_map_a_borrowed_query_and_a_value() {
        let mut fixture = MapFixture::default();
        let _ = fixture.write(1);
        let expected = vec![
            Type::mutable(CollectionFixtures::map()),
            Type::shared(CollectionFixtures::element()),
            Type::bool(),
        ];
        assert_eq!(fixture.parameters, [expected]);
    }

    #[test]
    fn a_map_write_checks_the_borrow_of_its_query() {
        let mut fixture = MapFixture::default();
        let _ = fixture.write(1);
        assert_eq!(fixture.borrows_checked, [CollectionFixtures::element()]);
    }
}
