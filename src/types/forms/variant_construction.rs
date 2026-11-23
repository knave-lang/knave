use super::{ExpectedChildTyping, ExpressionForm, VariantSites};
use crate::core::{BindingId, Expr};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::problems::TypeProblems;
use crate::types::ty::Type;
use crate::types::typed::{FieldOrder, TryErrorConversion, TypedExpr, TypedExprKind};

pub(in crate::types) struct VariantConstruction<'e> {
    binding: BindingId,
    arguments: &'e [Expr],
    span: Span,
}

impl<'e> VariantConstruction<'e> {
    pub(in crate::types) fn new(binding: BindingId, arguments: &'e [Expr], span: Span) -> Self {
        Self {
            binding,
            arguments,
            span,
        }
    }

    fn ok_of_try(typed: &[TypedExpr]) -> Option<TypedExpr> {
        let [
            TypedExpr {
                kind:
                    TypedExprKind::Try {
                        value,
                        conversion: TryErrorConversion::Identity,
                    },
                ..
            },
        ] = typed
        else {
            return None;
        };
        Some((**value).clone())
    }
}

impl<C> ExpressionForm<C> for VariantConstruction<'_>
where
    C: ExpectedChildTyping + VariantSites,
{
    fn type_against(
        &self,
        context: &mut C,
        hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let expected = hint.map(|hint| hint.expected);
        let site = context.variant_site(self.binding, expected);
        if self.arguments.len() != site.info.arity {
            return Err(TypeProblems::wrong_argument_count(
                site.info.arity,
                self.arguments.len(),
                self.span,
            ));
        }
        let mut typed = Vec::new();
        for (argument, expected) in self.arguments.iter().zip(&site.payload) {
            typed.push(context.type_child_against(argument, Expectation::plain(expected))?);
        }
        if context.is_result_ok(site.info)
            && let Some(value) = Self::ok_of_try(&typed)
        {
            return Ok(value);
        }
        Ok(TypedExpr {
            kind: TypedExprKind::Variant(self.binding, typed, FieldOrder::Declared),
            ty: Type::Named(site.info.owner, site.type_arguments),
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::VariantConstruction;
    use crate::core::{BindingId, Expr, ExprKind, Leaf, Literal, PrimitiveType, VariantInfo};
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::expectation::Expectation;
    use crate::types::forms::{ExpectedChildTyping, ExpressionForm, VariantSites};
    use crate::types::ty::Type;
    use crate::types::typed::{TryErrorConversion, TypedExpr, TypedExprKind, TypedLeaf};
    use crate::types::variant_site::VariantSite;

    struct VariantFixture {
        result_ok: bool,
        child_conversion: Option<TryErrorConversion>,
        site_expected: Option<Option<Type>>,
        payload_expected: Vec<Type>,
    }

    impl VariantFixture {
        const OWNER: BindingId = BindingId::from_index(1);
        const VARIANT: BindingId = BindingId::from_index(2);

        fn new() -> Self {
            Self {
                result_ok: false,
                child_conversion: None,
                site_expected: None,
                payload_expected: Vec::new(),
            }
        }

        fn construct(&mut self, arguments: usize) -> Result<TypedExpr, Diagnostic> {
            self.construct_with_hint(arguments, None)
        }

        fn construct_with_hint(
            &mut self,
            arguments: usize,
            hint: Option<&Type>,
        ) -> Result<TypedExpr, Diagnostic> {
            let arguments: Vec<Expr> = (0..arguments).map(|_| Self::argument()).collect();
            VariantConstruction::new(Self::VARIANT, &arguments, Self::span())
                .type_against(self, hint.map(Expectation::plain))
        }

        fn argument() -> Expr {
            Expr::new(
                ExprKind::Leaf(Leaf::Literal(Literal::Bool(true))),
                Self::span(),
            )
        }

        fn literal() -> TypedExpr {
            TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                ty: Type::bool(),
                span: Self::span(),
            }
        }

        fn payload() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn type_argument() -> Type {
            Type::Primitive(PrimitiveType::U8)
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }
    }

    impl ExpectedChildTyping for VariantFixture {
        fn type_child_against(
            &mut self,
            _child: &Expr,
            expectation: Expectation,
        ) -> Result<TypedExpr, Diagnostic> {
            self.payload_expected.push(expectation.expected.clone());
            let typed = match self.child_conversion {
                Some(conversion) => TypedExpr {
                    kind: TypedExprKind::Try {
                        value: Box::new(Self::literal()),
                        conversion,
                    },
                    ..Self::literal()
                },
                None => Self::literal(),
            };
            Ok(typed)
        }
    }

    impl VariantSites for VariantFixture {
        fn variant_site(&mut self, _binding: BindingId, expected: Option<&Type>) -> VariantSite {
            self.site_expected = Some(expected.cloned());
            VariantSite {
                info: VariantInfo {
                    owner: Self::OWNER,
                    index: 0,
                    arity: 1,
                },
                type_arguments: vec![Self::type_argument()],
                payload: vec![Self::payload()],
            }
        }

        fn is_result_ok(&self, _info: VariantInfo) -> bool {
            self.result_ok
        }
    }

    #[test]
    fn a_wrong_number_of_arguments_is_refused() {
        assert!(VariantFixture::new().construct(2).is_err());
    }

    #[test]
    fn each_argument_is_typed_against_its_payload_type() {
        let mut fixture = VariantFixture::new();
        let _ = fixture.construct(1);
        assert_eq!(fixture.payload_expected, [VariantFixture::payload()]);
    }

    #[test]
    fn the_hint_chooses_the_site() {
        let mut fixture = VariantFixture::new();
        let hint = VariantFixture::type_argument();
        let _ = fixture.construct_with_hint(1, Some(&hint));
        assert_eq!(fixture.site_expected, Some(Some(hint)));
    }

    #[test]
    fn a_variant_has_its_owner_and_type_arguments() {
        let typed = VariantFixture::new()
            .construct(1)
            .map(|typed| typed.ty)
            .ok();
        let owner = Type::Named(VariantFixture::OWNER, vec![VariantFixture::type_argument()]);
        assert_eq!(typed, Some(owner));
    }

    #[test]
    fn an_ok_of_an_unconverted_try_is_the_tried_value() {
        let mut fixture = VariantFixture::new();
        fixture.result_ok = true;
        fixture.child_conversion = Some(TryErrorConversion::Identity);
        let typed = fixture.construct(1).map(|typed| typed.ty).ok();
        assert_eq!(typed, Some(Type::bool()));
    }

    #[test]
    fn an_ok_of_a_converted_try_stays_a_variant() {
        let mut fixture = VariantFixture::new();
        fixture.result_ok = true;
        fixture.child_conversion = Some(TryErrorConversion::Converted);
        let typed = fixture.construct(1).map(|typed| typed.kind).ok();
        assert!(matches!(typed, Some(TypedExprKind::Variant(..))));
    }

    #[test]
    fn a_try_in_another_variant_stays_a_variant() {
        let mut fixture = VariantFixture::new();
        fixture.child_conversion = Some(TryErrorConversion::Identity);
        let typed = fixture.construct(1).map(|typed| typed.kind).ok();
        assert!(matches!(typed, Some(TypedExprKind::Variant(..))));
    }
}
