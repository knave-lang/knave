use super::constructor_tag::ConstructorTag;
use super::cover_pattern::CoverPattern;
use crate::types::Type;

pub(super) struct Constructor {
    tag: ConstructorTag,
    arguments: Vec<Type>,
}

impl Constructor {
    pub(super) fn new(tag: ConstructorTag, arguments: Vec<Type>) -> Self {
        Self { tag, arguments }
    }

    pub(super) fn tag(&self) -> ConstructorTag {
        self.tag
    }

    pub(super) fn arity(&self) -> usize {
        self.arguments.len()
    }

    pub(super) fn wild_arguments(&self) -> Vec<CoverPattern> {
        vec![CoverPattern::wild(); self.arity()]
    }

    pub(super) fn columns_with(&self, rest: &[Type]) -> Vec<Type> {
        [&self.arguments[..], rest].concat()
    }

    pub(super) fn arguments_of(&self, pattern: &CoverPattern) -> Option<Vec<CoverPattern>> {
        if pattern.is_wild() {
            return Some(self.wild_arguments());
        }
        (pattern.head_tag() == Some(self.tag)).then(|| pattern.arguments().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::Constructor;
    use crate::core::{BindingId, Literal, PrimitiveType};
    use crate::types::Type;
    use crate::types::coverage::constructor_tag::ConstructorTag;
    use crate::types::coverage::cover_pattern::CoverPattern;

    struct ConstructorFixture {
        owner: BindingId,
    }

    impl ConstructorFixture {
        fn new() -> Self {
            Self {
                owner: BindingId::from_index(0),
            }
        }

        fn pair(&self) -> Constructor {
            Constructor::new(
                ConstructorTag::Variant {
                    owner: self.owner,
                    index: 0,
                },
                vec![Self::boolean(), Self::text()],
            )
        }

        fn variant_pattern(&self, index: usize) -> CoverPattern {
            CoverPattern::constructed(
                ConstructorTag::Variant {
                    owner: self.owner,
                    index,
                },
                vec![
                    CoverPattern::from(Literal::Bool(true)),
                    CoverPattern::wild(),
                ],
            )
        }

        fn boolean() -> Type {
            Type::Primitive(PrimitiveType::Bool)
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn true_constructor() -> Constructor {
            Constructor::new(ConstructorTag::Bool(true), Vec::new())
        }
    }

    #[test]
    fn its_arity_counts_its_argument_types() {
        assert_eq!(ConstructorFixture::new().pair().arity(), 2);
    }

    #[test]
    fn its_wild_arguments_are_one_wildcard_per_argument() {
        let wild = ConstructorFixture::new().pair().wild_arguments();
        assert_eq!(wild.len(), 2);
        assert!(wild.iter().all(CoverPattern::is_wild));
    }

    #[test]
    fn its_columns_put_its_argument_types_before_the_rest() {
        let rest = [Type::Primitive(PrimitiveType::I64)];
        let columns = ConstructorFixture::new().pair().columns_with(&rest);
        assert_eq!(
            columns,
            vec![
                ConstructorFixture::boolean(),
                ConstructorFixture::text(),
                Type::Primitive(PrimitiveType::I64),
            ]
        );
    }

    #[test]
    fn a_wildcard_gives_it_one_wildcard_per_argument() {
        let arguments = ConstructorFixture::new()
            .pair()
            .arguments_of(&CoverPattern::wild())
            .expect("a wildcard fits every constructor");
        assert_eq!(arguments.len(), 2);
        assert!(arguments.iter().all(CoverPattern::is_wild));
    }

    #[test]
    fn a_pattern_with_its_tag_gives_it_the_pattern_arguments() {
        let fixture = ConstructorFixture::new();
        let arguments = fixture.pair().arguments_of(&fixture.variant_pattern(0));
        let arguments = arguments.expect("the pattern has the constructor's tag");
        assert!(arguments[0].head_tag() == Some(ConstructorTag::Bool(true)));
        assert!(arguments[1].is_wild());
    }

    #[test]
    fn a_pattern_with_another_tag_gives_it_nothing() {
        let fixture = ConstructorFixture::new();
        assert!(
            fixture
                .pair()
                .arguments_of(&fixture.variant_pattern(1))
                .is_none()
        );
    }

    #[test]
    fn a_bool_literal_gives_its_own_value_no_arguments() {
        let same = CoverPattern::from(Literal::Bool(true));
        let arguments = ConstructorFixture::true_constructor().arguments_of(&same);
        assert!(arguments.is_some_and(|arguments| arguments.is_empty()));
    }

    #[test]
    fn a_bool_literal_gives_the_other_value_nothing() {
        let other = CoverPattern::from(Literal::Bool(false));
        assert!(
            ConstructorFixture::true_constructor()
                .arguments_of(&other)
                .is_none()
        );
    }

    #[test]
    fn a_struct_pattern_with_its_tag_gives_it_the_pattern_fields() {
        let fixture = ConstructorFixture::new();
        let point = Constructor::new(
            ConstructorTag::Struct(fixture.owner),
            vec![ConstructorFixture::boolean()],
        );
        let pattern = CoverPattern::constructed(
            ConstructorTag::Struct(fixture.owner),
            vec![CoverPattern::from(Literal::Bool(false))],
        );
        let fields = point
            .arguments_of(&pattern)
            .expect("the pattern has the struct's tag");
        assert_eq!(fields.len(), 1);
        assert!(fields[0].head_tag() == Some(ConstructorTag::Bool(false)));
    }
}
