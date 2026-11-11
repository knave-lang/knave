use super::constructor_tag::ConstructorTag;
use crate::core::{BindingArena, BindingKind, Literal};
use crate::types::{TypedPattern, TypedPatternKind};

#[derive(Clone)]
pub(super) enum CoverPattern {
    Wild(WildPattern),
    Constructed(ConstructedPattern),
    Literal(LiteralPattern),
}

#[derive(Clone)]
pub(super) struct WildPattern;

#[derive(Clone)]
pub(super) struct ConstructedPattern {
    tag: ConstructorTag,
    arguments: Vec<CoverPattern>,
}

#[derive(Clone)]
pub(super) struct LiteralPattern {
    literal: Literal,
}

trait ColumnPattern {
    fn arguments(&self) -> &[CoverPattern];
    fn head_tag(&self) -> Option<ConstructorTag>;
    fn is_wild(&self) -> bool;
    fn literal(&self) -> Option<&Literal>;
    fn covers(&self, head: &dyn ColumnPattern) -> bool;
}

impl CoverPattern {
    pub(super) fn of(pattern: &TypedPattern, bindings: &BindingArena) -> Self {
        match &pattern.kind {
            TypedPatternKind::Wildcard | TypedPatternKind::Bind(_) => Self::wild(),
            TypedPatternKind::Variant(binding, arguments) => {
                let BindingKind::Variant(_, info) = bindings.get(*binding).kind() else {
                    unreachable!("a typed variant pattern names a variant binding");
                };
                let tag = ConstructorTag::Variant {
                    owner: info.owner,
                    index: info.index,
                };
                Self::constructed(tag, Self::all_of(arguments, bindings))
            }
            TypedPatternKind::Struct(binding, arguments) => Self::constructed(
                ConstructorTag::Struct(*binding),
                Self::all_of(arguments, bindings),
            ),
            TypedPatternKind::Literal(literal, _) => Self::from(literal.clone()),
        }
    }

    pub(super) fn wild() -> Self {
        Self::Wild(WildPattern)
    }

    pub(super) fn constructed(tag: ConstructorTag, arguments: Vec<CoverPattern>) -> Self {
        Self::Constructed(ConstructedPattern { tag, arguments })
    }

    pub(super) fn arguments(&self) -> &[CoverPattern] {
        self.kind().arguments()
    }

    pub(super) fn head_tag(&self) -> Option<ConstructorTag> {
        self.kind().head_tag()
    }

    pub(super) fn is_wild(&self) -> bool {
        self.kind().is_wild()
    }

    pub(super) fn covers(&self, head: &CoverPattern) -> bool {
        self.kind().covers(head.kind())
    }

    fn all_of(patterns: &[TypedPattern], bindings: &BindingArena) -> Vec<CoverPattern> {
        patterns
            .iter()
            .map(|pattern| Self::of(pattern, bindings))
            .collect()
    }

    fn kind(&self) -> &dyn ColumnPattern {
        match self {
            Self::Wild(pattern) => pattern,
            Self::Constructed(pattern) => pattern,
            Self::Literal(pattern) => pattern,
        }
    }
}

impl From<Literal> for CoverPattern {
    fn from(literal: Literal) -> Self {
        Self::Literal(LiteralPattern { literal })
    }
}

impl ColumnPattern for WildPattern {
    fn arguments(&self) -> &[CoverPattern] {
        &[]
    }

    fn head_tag(&self) -> Option<ConstructorTag> {
        None
    }

    fn is_wild(&self) -> bool {
        true
    }

    fn literal(&self) -> Option<&Literal> {
        None
    }

    fn covers(&self, _head: &dyn ColumnPattern) -> bool {
        true
    }
}

impl ColumnPattern for ConstructedPattern {
    fn arguments(&self) -> &[CoverPattern] {
        &self.arguments
    }

    fn head_tag(&self) -> Option<ConstructorTag> {
        Some(self.tag)
    }

    fn is_wild(&self) -> bool {
        false
    }

    fn literal(&self) -> Option<&Literal> {
        None
    }

    fn covers(&self, _head: &dyn ColumnPattern) -> bool {
        false
    }
}

impl ColumnPattern for LiteralPattern {
    fn arguments(&self) -> &[CoverPattern] {
        &[]
    }

    fn head_tag(&self) -> Option<ConstructorTag> {
        match self.literal {
            Literal::Bool(value) => Some(ConstructorTag::Bool(value)),
            _ => None,
        }
    }

    fn is_wild(&self) -> bool {
        false
    }

    fn literal(&self) -> Option<&Literal> {
        Some(&self.literal)
    }

    fn covers(&self, head: &dyn ColumnPattern) -> bool {
        match (&self.literal, head.literal()) {
            (Literal::Number(left), Some(Literal::Number(right))) => left.value() == right.value(),
            (Literal::String(left), Some(Literal::String(right))) => left == right,
            (Literal::Bool(left), Some(Literal::Bool(right))) => left == right,
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
    use crate::types::coverage::constructor_tag::ConstructorTag;
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
        assert!(pattern.is_wild());
    }

    #[test]
    fn a_variant_covers_with_the_owner_and_index_of_its_binding() {
        let fixture = PatternFixture::new();
        let pattern = fixture.circle_of(TypedPatternKind::Wildcard);
        let tag = ConstructorTag::Variant {
            owner: fixture.owner,
            index: 1,
        };
        assert!(pattern.head_tag() == Some(tag));
    }

    #[test]
    fn a_variant_keeps_its_argument_patterns() {
        let literal =
            TypedPatternKind::Literal(PatternFixture::text("unit"), PatternFixture::string_type());
        let pattern = PatternFixture::new().circle_of(literal);
        let unit = CoverPattern::from(PatternFixture::text("unit"));
        assert_eq!(pattern.arguments().len(), 1);
        assert!(pattern.arguments()[0].covers(&unit));
        assert!(!pattern.arguments()[0].is_wild());
    }

    #[test]
    fn a_literal_has_no_arguments() {
        assert!(
            CoverPattern::from(Literal::Bool(true))
                .arguments()
                .is_empty()
        );
    }

    #[test]
    fn a_wildcard_matches_every_literal() {
        let any = CoverPattern::from(PatternFixture::text("any"));
        assert!(CoverPattern::wild().covers(&any));
    }

    #[test]
    fn a_string_matches_only_the_same_string() {
        let pattern = CoverPattern::from(PatternFixture::text("north"));
        assert!(pattern.covers(&CoverPattern::from(PatternFixture::text("north"))));
        assert!(!pattern.covers(&CoverPattern::from(PatternFixture::text("south"))));
    }

    #[test]
    fn a_literal_never_matches_one_of_another_kind() {
        let pattern = CoverPattern::from(Literal::Bool(true));
        assert!(!pattern.covers(&CoverPattern::from(PatternFixture::text("true"))));
    }

    #[test]
    fn a_binding_covers_as_a_wildcard() {
        let fixture = PatternFixture::new();
        let pattern = CoverPattern::of(&PatternFixture::typed(fixture.binder()), &fixture.bindings);
        assert!(pattern.is_wild());
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
        assert!(pattern.head_tag() == Some(ConstructorTag::Struct(fixture.owner)));
        assert_eq!(pattern.arguments().len(), 1);
        assert!(pattern.arguments()[0].head_tag() == Some(ConstructorTag::Bool(true)));
    }

    #[test]
    fn a_number_matches_only_the_same_number() {
        let pattern = CoverPattern::from(PatternFixture::number(7));
        assert!(pattern.covers(&CoverPattern::from(PatternFixture::number(7))));
        assert!(!pattern.covers(&CoverPattern::from(PatternFixture::number(8))));
    }

    #[test]
    fn a_bool_matches_only_the_same_bool() {
        let pattern = CoverPattern::from(Literal::Bool(true));
        assert!(pattern.covers(&CoverPattern::from(Literal::Bool(true))));
        assert!(!pattern.covers(&CoverPattern::from(Literal::Bool(false))));
    }
}
