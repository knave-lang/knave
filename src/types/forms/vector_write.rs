use super::VectorWriteTyping;
use crate::core::KeyedPath;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::expectation::Expectation;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::AssignmentPath;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct VectorWrite<'e> {
    path: &'e KeyedPath,
    span: Span,
}

impl<'e> VectorWrite<'e> {
    pub(in crate::types) fn new(path: &'e KeyedPath, span: Span) -> Self {
        Self { path, span }
    }

    pub(in crate::types) fn type_with<C: VectorWriteTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
        collection: Collection,
    ) -> Result<TypedExpr, Diagnostic> {
        let depth = self.path.keys.len();
        let item_type = context.fresh();
        let target = Self::target(context, depth, &item_type);
        context.fit(Expectation::plain(&target), collection.ty(), value.span)?;
        let mut parameters = vec![Type::mutable(target)];
        parameters.extend((0..depth).map(|_| Collection::INDEX));
        parameters.push(item_type);
        let template = SyntheticTemplate::set_at(depth);
        let arguments = self.path.keys_and_item();
        let call = SyntheticCall {
            signature: SyntheticSignature {
                template: &template,
                parameters,
                result: Type::unit(),
            },
            value,
            arguments: &arguments,
            span: self.span,
        };
        context.type_place_write(call, AssignmentPath::Elements)
    }

    fn target<C: VectorWriteTyping>(context: &C, depth: usize, item: &Type) -> Type {
        match depth.checked_sub(1) {
            Some(row_depth) => context.slice_of(context.nested_vector_of(row_depth, item)),
            None => item.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VectorWrite;
    use crate::core::{Expr, KeyedPath, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::collection::Collection;
    use crate::types::expectation::Expectation;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{PlaceWrites, PreludeShapes, TypeFitting, Unknowns};
    use crate::types::problems::TypeProblems;
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_template::SyntheticTemplate;
    use crate::types::ty::Type;
    use crate::types::typed::AssignmentPath;
    use crate::types::typed::TypedExpr;

    #[derive(Default)]
    struct VectorFixture {
        refuses_base: bool,
        fitted: Vec<Type>,
        calls: Vec<(String, Vec<Type>, Type)>,
    }

    impl VectorFixture {
        fn write(&mut self, keys: usize) -> Result<TypedExpr, Diagnostic> {
            let path = KeyedPath {
                value: Box::new(Self::truth()),
                keys: (0..keys).map(|_| Self::truth()).collect(),
                item: Box::new(Self::truth()),
            };
            let collections = CollectionFixtures::new();
            let base = CollectionFixtures::vector_of(Type::bool());
            let value = FormFixtures::typed_leaf(Literal::Bool(true), base.clone());
            VectorWrite::new(&path, FormFixtures::span()).type_with(
                self,
                value,
                collections.collection(&base),
            )
        }

        fn truth() -> Expr {
            FormFixtures::leaf(Literal::Bool(true))
        }

        fn row_target() -> Type {
            CollectionFixtures::slice_of(CollectionFixtures::vector_of(
                CollectionFixtures::element(),
            ))
        }
    }

    impl Unknowns for VectorFixture {
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

    impl PreludeShapes for VectorFixture {
        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }

        fn slice_of(&self, element: Type) -> Type {
            CollectionFixtures::slice_of(element)
        }

        fn option_of(&self, _value: Type) -> Type {
            unreachable!("this form builds no option")
        }

        fn iterator_of(&self, _item: Type) -> Type {
            unreachable!("this form builds no iterator")
        }
    }

    impl TypeFitting for VectorFixture {
        fn fit(
            &mut self,
            expectation: Expectation,
            _found: &Type,
            span: Span,
        ) -> Result<(), Diagnostic> {
            self.fitted.push(expectation.expected.clone());
            if self.refuses_base {
                return Err(TypeProblems::wrong_argument_count(1, 0, span));
            }
            Ok(())
        }
    }

    impl PlaceWrites for VectorFixture {
        fn type_place_write(
            &mut self,
            call: SyntheticCall<'_>,
            _path: AssignmentPath,
        ) -> Result<TypedExpr, Diagnostic> {
            self.calls.push((
                call.signature.template.to_string(),
                call.signature.parameters,
                call.signature.result,
            ));
            Ok(call.value)
        }
    }

    #[test]
    fn a_one_key_write_fits_the_base_to_a_slice_of_items() {
        let mut fixture = VectorFixture::default();
        let _ = fixture.write(1);
        let expected = CollectionFixtures::slice_of(CollectionFixtures::element());
        assert_eq!(fixture.fitted, [expected]);
    }

    #[test]
    fn a_two_key_write_fits_the_base_to_a_slice_of_rows() {
        let mut fixture = VectorFixture::default();
        let _ = fixture.write(2);
        assert_eq!(fixture.fitted, [VectorFixture::row_target()]);
    }

    #[test]
    fn a_zero_key_write_fits_the_base_to_the_item() {
        let mut fixture = VectorFixture::default();
        let _ = fixture.write(0);
        assert_eq!(fixture.fitted, [CollectionFixtures::element()]);
    }

    #[test]
    fn a_refused_base_refuses_the_write() {
        let mut fixture = VectorFixture {
            refuses_base: true,
            ..Default::default()
        };
        assert!(fixture.write(1).is_err());
    }

    #[test]
    fn a_vector_write_passes_an_index_per_key_then_the_item() {
        let mut fixture = VectorFixture::default();
        let _ = fixture.write(2);
        let expected = vec![
            Type::mutable(VectorFixture::row_target()),
            Collection::INDEX,
            Collection::INDEX,
            CollectionFixtures::element(),
        ];
        assert!(
            matches!(fixture.calls.as_slice(), [(_, parameters, _)] if *parameters == expected)
        );
    }

    #[test]
    fn a_vector_write_calls_the_set_at_template_of_its_depth() {
        let mut fixture = VectorFixture::default();
        let _ = fixture.write(2);
        let expected = SyntheticTemplate::set_at(2);
        assert!(matches!(fixture.calls.as_slice(), [(template, _, _)] if *template == expected));
    }

    #[test]
    fn a_vector_write_is_unit() {
        let mut fixture = VectorFixture::default();
        let _ = fixture.write(1);
        assert!(matches!(fixture.calls.as_slice(), [(_, _, result)] if *result == Type::unit()));
    }
}
