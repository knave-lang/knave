use super::SyntheticCalls;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::ClosureKind;
use crate::types::collection::Collection;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct MapUpdate<'e> {
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> MapUpdate<'e> {
    const ARGUMENTS: usize = 3;

    pub(in crate::types) fn new(arguments: &'e [Expr], span: Span) -> Self {
        Self { arguments, span }
    }

    pub(in crate::types) fn type_with<C: SyntheticCalls>(
        &self,
        context: &mut C,
        value: TypedExpr,
        map: Collection,
    ) -> Result<TypedExpr, Diagnostic> {
        if self.arguments.len() != Self::ARGUMENTS {
            let found = self.arguments.len();
            return Err(TypeProblems::wrong_argument_count(
                Self::ARGUMENTS,
                found,
                self.span,
            ));
        }
        let value_type = map.value().clone();
        let parameters = vec![
            map.ty().clone(),
            map.key().clone(),
            Self::updater(&value_type),
            value_type,
        ];
        context.type_synthetic_call(SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::MAP_UPDATE.text(),
                parameters,
                result: map.ty().clone(),
            },
            value,
            arguments: self.arguments,
            span: self.span,
        })
    }

    fn updater(value_type: &Type) -> Type {
        Type::Function(
            ClosureKind::Fn,
            vec![value_type.clone()],
            Box::new(value_type.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MapUpdate;
    use crate::core::{Expr, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::ClosureKind;
    use crate::types::forms::SyntheticCalls;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct UpdateFixture {
        calls: Vec<(Vec<Type>, Type)>,
    }

    impl UpdateFixture {
        fn update(&mut self, arguments: usize) -> Result<TypedExpr, Diagnostic> {
            let arguments: Vec<Expr> = (0..arguments)
                .map(|_| FormFixtures::leaf(Literal::Bool(true)))
                .collect();
            let map = CollectionFixtures::new().collection(&CollectionFixtures::map());
            let value = FormFixtures::typed_leaf(Literal::Bool(true), CollectionFixtures::map());
            MapUpdate::new(&arguments, FormFixtures::span()).type_with(self, value, map)
        }
    }

    impl SyntheticCalls for UpdateFixture {
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
    fn an_update_without_three_arguments_is_refused() {
        let mut fixture = UpdateFixture::default();
        assert!(fixture.update(2).is_err());
        assert!(fixture.calls.is_empty());
    }

    #[test]
    fn an_update_takes_the_map_its_key_an_updater_and_a_default() {
        let mut fixture = UpdateFixture::default();
        let _ = fixture.update(3);
        let updater = Type::Function(ClosureKind::Fn, vec![Type::bool()], Box::new(Type::bool()));
        let expected = vec![
            CollectionFixtures::map(),
            CollectionFixtures::key(),
            updater,
            Type::bool(),
        ];
        assert!(matches!(fixture.calls.as_slice(), [(parameters, _)] if *parameters == expected));
    }

    #[test]
    fn an_update_gives_the_map() {
        let mut fixture = UpdateFixture::default();
        let _ = fixture.update(3);
        let expected = CollectionFixtures::map();
        assert!(matches!(fixture.calls.as_slice(), [(_, result)] if *result == expected));
    }
}
