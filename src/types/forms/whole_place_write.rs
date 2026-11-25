use super::WholePlaceWriteTyping;
use crate::core::KeyedPath;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::{AssignmentPath, TypedExpr};

pub(in crate::types) struct WholePlaceWrite<'e> {
    path: &'e KeyedPath,
    span: Span,
}

impl<'e> WholePlaceWrite<'e> {
    pub(in crate::types) fn new(path: &'e KeyedPath, span: Span) -> Self {
        Self { path, span }
    }

    pub(in crate::types) fn type_with<C: WholePlaceWriteTyping>(
        &self,
        context: &mut C,
        place: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let item = context.fresh();
        let arguments = self.path.keys_and_item();
        let call = SyntheticCall {
            signature: SyntheticSignature {
                template: SyntheticTemplate::ASSIGN.text(),
                parameters: vec![Type::mutable(item.clone()), item],
                result: Type::unit(),
            },
            value: place,
            arguments: &arguments,
            span: self.span,
        };
        context.type_place_write(call, AssignmentPath::Whole)
    }
}

#[cfg(test)]
mod tests {
    use super::WholePlaceWrite;
    use crate::core::{Expr, KeyedPath, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{PlaceWrites, Unknowns};
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_template::SyntheticTemplate;
    use crate::types::ty::{Type, TypeVar};
    use crate::types::typed::{AssignmentPath, TypedExpr};

    #[derive(Default)]
    struct WholeFixture {
        writes: Vec<(String, Vec<Type>, AssignmentPath)>,
    }

    impl WholeFixture {
        fn write(&mut self) -> Result<TypedExpr, Diagnostic> {
            let path = KeyedPath {
                value: Box::new(Self::truth()),
                keys: Vec::new(),
                item: Box::new(Self::truth()),
            };
            let place = FormFixtures::typed_leaf(Literal::Bool(true), Type::bool());
            WholePlaceWrite::new(&path, FormFixtures::span()).type_with(self, place)
        }

        fn truth() -> Expr {
            FormFixtures::leaf(Literal::Bool(true))
        }

        fn item() -> Type {
            Type::Var(TypeVar::new(0))
        }
    }

    impl Unknowns for WholeFixture {
        fn fresh(&mut self) -> Type {
            Self::item()
        }

        fn resolve(&self, value: &Type) -> Type {
            value.clone()
        }

        fn is_open(&self, _value: &Type) -> bool {
            false
        }
    }

    impl PlaceWrites for WholeFixture {
        fn type_place_write(
            &mut self,
            call: SyntheticCall<'_>,
            path: AssignmentPath,
        ) -> Result<TypedExpr, Diagnostic> {
            let template = call.signature.template.to_string();
            self.writes
                .push((template, call.signature.parameters, path));
            Ok(call.value)
        }
    }

    #[test]
    fn a_whole_place_write_assigns_through_the_place() {
        let mut fixture = WholeFixture::default();
        let _ = fixture.write();
        let (template, _, path) = &fixture.writes[0];
        assert_eq!(template, SyntheticTemplate::ASSIGN.text());
        assert_eq!(*path, AssignmentPath::Whole);
    }

    #[test]
    fn a_whole_place_write_passes_the_place_and_the_item_one_type() {
        let mut fixture = WholeFixture::default();
        let _ = fixture.write();
        let (_, parameters, _) = &fixture.writes[0];
        let item = WholeFixture::item();
        assert_eq!(*parameters, [Type::mutable(item.clone()), item]);
    }
}
