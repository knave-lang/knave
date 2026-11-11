use crate::core::{BindingArena, BindingId, BindingKind, Literal};
use crate::types::{TypedPattern, TypedPatternKind};

#[derive(Clone)]
pub(super) enum CoverPattern {
    Wild,
    Variant {
        owner: BindingId,
        index: usize,
        arguments: Vec<CoverPattern>,
    },
    Struct {
        owner: BindingId,
        arguments: Vec<CoverPattern>,
    },
    Literal(Literal),
}

impl CoverPattern {
    pub(super) fn of(pattern: &TypedPattern, bindings: &BindingArena) -> Self {
        match &pattern.kind {
            TypedPatternKind::Wildcard | TypedPatternKind::Bind(_) => CoverPattern::Wild,
            TypedPatternKind::Variant(binding, arguments) => {
                let BindingKind::Variant(_, info) = bindings.get(*binding).kind() else {
                    unreachable!("a typed variant pattern names a variant binding");
                };
                CoverPattern::Variant {
                    owner: info.owner,
                    index: info.index,
                    arguments: Self::all_of(arguments, bindings),
                }
            }
            TypedPatternKind::Struct(binding, arguments) => CoverPattern::Struct {
                owner: *binding,
                arguments: Self::all_of(arguments, bindings),
            },
            TypedPatternKind::Literal(literal, _) => CoverPattern::Literal(literal.clone()),
        }
    }

    fn all_of(patterns: &[TypedPattern], bindings: &BindingArena) -> Vec<CoverPattern> {
        patterns
            .iter()
            .map(|pattern| Self::of(pattern, bindings))
            .collect()
    }

    pub(super) fn arguments(&self) -> &[CoverPattern] {
        match self {
            CoverPattern::Variant { arguments, .. } | CoverPattern::Struct { arguments, .. } => {
                arguments
            }
            CoverPattern::Wild | CoverPattern::Literal(_) => &[],
        }
    }

    pub(super) fn matches_literal(&self, literal: &Literal) -> bool {
        match (self, literal) {
            (CoverPattern::Wild, _) => true,
            (CoverPattern::Literal(Literal::Number(left)), Literal::Number(right)) => {
                left.value() == right.value()
            }
            (CoverPattern::Literal(Literal::String(left)), Literal::String(right)) => left == right,
            (CoverPattern::Literal(Literal::Bool(left)), Literal::Bool(right)) => left == right,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CoverPattern;
    use crate::core::{
        BindingArena, BindingId, BindingInfo, BindingKind, Literal, ModuleId, PrimitiveType,
        VariantInfo,
    };
    use crate::reader::{Number, NumberValue};
    use crate::source::{SourceId, Span};
    use crate::types::{Type, TypedBinder, TypedPattern, TypedPatternKind};

    struct PatternFixture {
        bindings: BindingArena,
        owner: BindingId,
        variant: BindingId,
    }

    impl PatternFixture {
        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let owner = bindings.add(BindingInfo::new(
                "shape".into(),
                BindingKind::Enum(ModuleId::new(0)),
                None,
            ));
            let info = VariantInfo {
                owner,
                index: 1,
                arity: 1,
            };
            let variant = bindings.add(BindingInfo::new(
                "circle".into(),
                BindingKind::Variant(ModuleId::new(0), info),
                None,
            ));
            Self {
                bindings,
                owner,
                variant,
            }
        }

        fn typed(kind: TypedPatternKind) -> TypedPattern {
            TypedPattern {
                kind,
                boxed: false,
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn circle_of(&self, argument: TypedPatternKind) -> CoverPattern {
            let pattern = TypedPatternKind::Variant(self.variant, vec![Self::typed(argument)]);
            CoverPattern::of(&Self::typed(pattern), &self.bindings)
        }

        fn text(text: &str) -> Literal {
            Literal::String(text.into())
        }

        fn string_type() -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn number(value: i128) -> Literal {
            Literal::Number(Number::new(value.to_string(), NumberValue::Integer(value)))
        }

        fn binder(&self) -> TypedPatternKind {
            TypedPatternKind::Bind(TypedBinder {
                binding: self.variant,
                ty: Self::string_type(),
                span: Span::new(SourceId::new(0), 0..0),
                annotated: false,
            })
        }
    }

    #[test]
    fn a_wildcard_covers_as_a_wildcard() {
        let fixture = PatternFixture::new();
        let pattern = CoverPattern::of(
            &PatternFixture::typed(TypedPatternKind::Wildcard),
            &fixture.bindings,
        );
        assert!(matches!(pattern, CoverPattern::Wild));
    }

    #[test]
    fn a_variant_covers_with_the_owner_and_index_of_its_binding() {
        let fixture = PatternFixture::new();
        let pattern = fixture.circle_of(TypedPatternKind::Wildcard);
        assert!(matches!(
            pattern,
            CoverPattern::Variant { owner, index: 1, .. } if owner == fixture.owner
        ));
    }

    #[test]
    fn a_variant_keeps_its_argument_patterns() {
        let literal =
            TypedPatternKind::Literal(PatternFixture::text("unit"), PatternFixture::string_type());
        let pattern = PatternFixture::new().circle_of(literal);
        assert!(matches!(
            pattern.arguments(),
            [CoverPattern::Literal(Literal::String(_))]
        ));
    }

    #[test]
    fn a_literal_has_no_arguments() {
        assert!(
            CoverPattern::Literal(Literal::Bool(true))
                .arguments()
                .is_empty()
        );
    }

    #[test]
    fn a_wildcard_matches_every_literal() {
        assert!(CoverPattern::Wild.matches_literal(&PatternFixture::text("any")));
    }

    #[test]
    fn a_string_matches_only_the_same_string() {
        let pattern = CoverPattern::Literal(PatternFixture::text("north"));
        assert!(pattern.matches_literal(&PatternFixture::text("north")));
        assert!(!pattern.matches_literal(&PatternFixture::text("south")));
    }

    #[test]
    fn a_literal_never_matches_one_of_another_kind() {
        let pattern = CoverPattern::Literal(Literal::Bool(true));
        assert!(!pattern.matches_literal(&PatternFixture::text("true")));
    }

    #[test]
    fn a_binding_covers_as_a_wildcard() {
        let fixture = PatternFixture::new();
        let pattern = CoverPattern::of(&PatternFixture::typed(fixture.binder()), &fixture.bindings);
        assert!(matches!(pattern, CoverPattern::Wild));
    }

    #[test]
    fn a_struct_covers_with_its_binding_and_field_patterns() {
        let fixture = PatternFixture::new();
        let field = PatternFixture::typed(TypedPatternKind::Literal(
            Literal::Bool(true),
            Type::Primitive(PrimitiveType::Bool),
        ));
        let pattern = CoverPattern::of(
            &PatternFixture::typed(TypedPatternKind::Struct(fixture.owner, vec![field])),
            &fixture.bindings,
        );
        assert!(matches!(
            &pattern,
            CoverPattern::Struct { owner, arguments } if *owner == fixture.owner
                && matches!(arguments.as_slice(), [CoverPattern::Literal(Literal::Bool(true))])
        ));
    }

    #[test]
    fn a_number_matches_only_the_same_number() {
        let pattern = CoverPattern::Literal(PatternFixture::number(7));
        assert!(pattern.matches_literal(&PatternFixture::number(7)));
        assert!(!pattern.matches_literal(&PatternFixture::number(8)));
    }

    #[test]
    fn a_bool_matches_only_the_same_bool() {
        let pattern = CoverPattern::Literal(Literal::Bool(true));
        assert!(pattern.matches_literal(&Literal::Bool(true)));
        assert!(!pattern.matches_literal(&Literal::Bool(false)));
    }
}
