use super::cover_pattern::CoverPattern;
use crate::core::{BindingId, Literal};
use crate::types::Type;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tag {
    Variant { owner: BindingId, index: usize },
    Struct(BindingId),
    Bool(bool),
}

pub(super) struct Constructor {
    tag: Tag,
    arguments: Vec<Type>,
}

impl Constructor {
    pub(super) fn new(tag: Tag, arguments: Vec<Type>) -> Self {
        Self { tag, arguments }
    }

    pub(super) fn tag(&self) -> Tag {
        self.tag
    }

    pub(super) fn arity(&self) -> usize {
        self.arguments.len()
    }

    pub(super) fn wild_arguments(&self) -> Vec<CoverPattern> {
        vec![CoverPattern::Wild; self.arity()]
    }

    pub(super) fn columns_with(&self, rest: &[Type]) -> Vec<Type> {
        [&self.arguments[..], rest].concat()
    }

    pub(super) fn arguments_of(&self, pattern: &CoverPattern) -> Option<Vec<CoverPattern>> {
        match pattern {
            CoverPattern::Wild => Some(self.wild_arguments()),
            CoverPattern::Variant {
                owner,
                index,
                arguments,
            } if self.tag
                == (Tag::Variant {
                    owner: *owner,
                    index: *index,
                }) =>
            {
                Some(arguments.clone())
            }
            CoverPattern::Struct { owner, arguments } if self.tag == Tag::Struct(*owner) => {
                Some(arguments.clone())
            }
            CoverPattern::Literal(Literal::Bool(value)) if self.tag == Tag::Bool(*value) => {
                Some(Vec::new())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Constructor, Tag};
    use crate::core::{BindingId, Literal, PrimitiveType};
    use crate::types::Type;
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
                Tag::Variant {
                    owner: self.owner,
                    index: 0,
                },
                vec![Self::boolean(), Self::text()],
            )
        }

        fn variant_pattern(&self, index: usize) -> CoverPattern {
            CoverPattern::Variant {
                owner: self.owner,
                index,
                arguments: vec![
                    CoverPattern::Literal(Literal::Bool(true)),
                    CoverPattern::Wild,
                ],
            }
        }

        fn boolean() -> Type {
            Type::Primitive(PrimitiveType::Bool)
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn true_constructor() -> Constructor {
            Constructor::new(Tag::Bool(true), Vec::new())
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
        assert!(
            wild.iter()
                .all(|pattern| matches!(pattern, CoverPattern::Wild))
        );
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
            .arguments_of(&CoverPattern::Wild);
        assert!(matches!(
            arguments.as_deref(),
            Some([CoverPattern::Wild, CoverPattern::Wild])
        ));
    }

    #[test]
    fn a_pattern_with_its_tag_gives_it_the_pattern_arguments() {
        let fixture = ConstructorFixture::new();
        let arguments = fixture.pair().arguments_of(&fixture.variant_pattern(0));
        assert!(matches!(
            arguments.as_deref(),
            Some([
                CoverPattern::Literal(Literal::Bool(true)),
                CoverPattern::Wild
            ])
        ));
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
        let same = CoverPattern::Literal(Literal::Bool(true));
        let arguments = ConstructorFixture::true_constructor().arguments_of(&same);
        assert!(matches!(arguments.as_deref(), Some([])));
    }

    #[test]
    fn a_bool_literal_gives_the_other_value_nothing() {
        let other = CoverPattern::Literal(Literal::Bool(false));
        assert!(
            ConstructorFixture::true_constructor()
                .arguments_of(&other)
                .is_none()
        );
    }
}
