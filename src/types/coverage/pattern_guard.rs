use crate::core::{Literal, PrimitiveType};
use crate::types::{Type, TypedPattern, TypedPatternKind};

pub(super) struct PatternGuard<'p> {
    pattern: &'p TypedPattern,
}

impl<'p> PatternGuard<'p> {
    pub(super) fn of(pattern: &'p TypedPattern) -> Self {
        Self { pattern }
    }

    pub(super) fn is_needed(&self) -> bool {
        if self.pattern.boxed
            && matches!(
                &self.pattern.kind,
                TypedPatternKind::Variant(..) | TypedPatternKind::Literal(..)
            )
        {
            return true;
        }
        match &self.pattern.kind {
            TypedPatternKind::Wildcard | TypedPatternKind::Bind(_) => false,
            TypedPatternKind::Variant(_, arguments) | TypedPatternKind::Struct(_, arguments) => {
                arguments
                    .iter()
                    .any(|argument| PatternGuard::of(argument).is_needed())
            }
            TypedPatternKind::Literal(literal, ty) => {
                matches!(literal, Literal::String(_))
                    && matches!(ty, Type::Primitive(PrimitiveType::String))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PatternGuard;
    use crate::core::{BindingId, Literal, PrimitiveType};
    use crate::source::{SourceId, Span};
    use crate::types::{Type, TypedPattern, TypedPatternKind};

    struct GuardFixture {
        variant: BindingId,
    }

    impl GuardFixture {
        fn new() -> Self {
            Self {
                variant: BindingId::from_index(0),
            }
        }

        fn typed(kind: TypedPatternKind) -> TypedPattern {
            TypedPattern {
                kind,
                boxed: false,
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn boxed(kind: TypedPatternKind) -> TypedPattern {
            TypedPattern {
                boxed: true,
                ..Self::typed(kind)
            }
        }

        fn variant_of(&self, arguments: Vec<TypedPattern>) -> TypedPatternKind {
            TypedPatternKind::Variant(self.variant, arguments)
        }

        fn string_literal() -> TypedPattern {
            Self::typed(TypedPatternKind::Literal(
                Literal::String("north".into()),
                Type::Primitive(PrimitiveType::String),
            ))
        }

        fn is_needed(pattern: &TypedPattern) -> bool {
            PatternGuard::of(pattern).is_needed()
        }
    }

    #[test]
    fn a_wildcard_needs_no_guard() {
        let wildcard = GuardFixture::typed(TypedPatternKind::Wildcard);
        assert!(!GuardFixture::is_needed(&wildcard));
    }

    #[test]
    fn a_variant_of_plain_patterns_needs_no_guard() {
        let fixture = GuardFixture::new();
        let wildcard = GuardFixture::typed(TypedPatternKind::Wildcard);
        let variant = GuardFixture::typed(fixture.variant_of(vec![wildcard]));
        assert!(!GuardFixture::is_needed(&variant));
    }

    #[test]
    fn a_boxed_variant_needs_a_guard() {
        let fixture = GuardFixture::new();
        let boxed = GuardFixture::boxed(fixture.variant_of(Vec::new()));
        assert!(GuardFixture::is_needed(&boxed));
    }

    #[test]
    fn a_string_literal_of_type_string_needs_a_guard() {
        assert!(GuardFixture::is_needed(&GuardFixture::string_literal()));
    }

    #[test]
    fn a_bool_literal_needs_no_guard() {
        let literal = GuardFixture::typed(TypedPatternKind::Literal(
            Literal::Bool(true),
            Type::Primitive(PrimitiveType::Bool),
        ));
        assert!(!GuardFixture::is_needed(&literal));
    }

    #[test]
    fn a_struct_needs_the_guard_of_a_field_pattern() {
        let fixture = GuardFixture::new();
        let boxed = GuardFixture::boxed(fixture.variant_of(Vec::new()));
        let holder = GuardFixture::typed(TypedPatternKind::Struct(
            BindingId::from_index(1),
            vec![boxed],
        ));
        assert!(GuardFixture::is_needed(&holder));
    }

    #[test]
    fn a_variant_needs_the_guard_of_an_argument_pattern() {
        let fixture = GuardFixture::new();
        let variant = GuardFixture::typed(fixture.variant_of(vec![GuardFixture::string_literal()]));
        assert!(GuardFixture::is_needed(&variant));
    }

    #[test]
    fn one_guarded_argument_among_plain_ones_needs_a_guard() {
        let fixture = GuardFixture::new();
        let wildcard = GuardFixture::typed(TypedPatternKind::Wildcard);
        let variant =
            GuardFixture::typed(fixture.variant_of(vec![wildcard, GuardFixture::string_literal()]));
        assert!(GuardFixture::is_needed(&variant));
    }
}
