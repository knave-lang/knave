use super::constructor::Constructor;
use super::cover_pattern::CoverPattern;
use crate::types::Type;

#[derive(Clone, Copy)]
pub(super) struct Query<'q> {
    patterns: &'q [CoverPattern],
    types: &'q [Type],
}

pub(super) struct ExpandedQuery {
    patterns: Vec<CoverPattern>,
    types: Vec<Type>,
}

impl<'q> Query<'q> {
    pub(super) fn new(patterns: &'q [CoverPattern], types: &'q [Type]) -> Self {
        Self { patterns, types }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    pub(super) fn first_pattern(&self) -> &'q CoverPattern {
        &self.patterns[0]
    }

    pub(super) fn first_type(&self) -> &'q Type {
        &self.types[0]
    }

    pub(super) fn rest(&self) -> Self {
        Self::new(&self.patterns[1..], &self.types[1..])
    }

    pub(super) fn expanded(
        &self,
        constructor: &Constructor,
        arguments: &[CoverPattern],
    ) -> ExpandedQuery {
        ExpandedQuery {
            patterns: [arguments, &self.patterns[1..]].concat(),
            types: constructor.columns_with(&self.types[1..]),
        }
    }
}

impl ExpandedQuery {
    pub(super) fn query(&self) -> Query<'_> {
        Query::new(&self.patterns, &self.types)
    }
}

#[cfg(test)]
mod tests {
    use super::Query;
    use crate::core::{Literal, PrimitiveType};
    use crate::types::Type;
    use crate::types::coverage::constructor::Constructor;
    use crate::types::coverage::constructor_tag::ConstructorTag;
    use crate::types::coverage::cover_pattern::CoverPattern;

    struct QueryFixture {
        patterns: Vec<CoverPattern>,
        types: Vec<Type>,
    }

    impl QueryFixture {
        fn new() -> Self {
            Self {
                patterns: vec![
                    CoverPattern::wild(),
                    CoverPattern::from(Literal::Bool(false)),
                ],
                types: vec![Self::text(), Self::boolean()],
            }
        }

        fn query(&self) -> Query<'_> {
            Query::new(&self.patterns, &self.types)
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn boolean() -> Type {
            Type::Primitive(PrimitiveType::Bool)
        }

        fn unit() -> Type {
            Type::Primitive(PrimitiveType::Unit)
        }
    }

    #[test]
    fn a_query_with_no_columns_is_empty() {
        assert!(Query::new(&[], &[]).is_empty());
    }

    #[test]
    fn its_first_column_gives_the_first_pattern_and_type() {
        let fixture = QueryFixture::new();
        assert!(fixture.query().first_pattern().is_wild());
        assert_eq!(fixture.query().first_type(), &QueryFixture::text());
    }

    #[test]
    fn its_rest_drops_the_first_column() {
        let fixture = QueryFixture::new();
        let rest = fixture.query().rest();
        assert!(rest.first_pattern().head_tag() == Some(ConstructorTag::Bool(false)));
        assert_eq!(rest.first_type(), &QueryFixture::boolean());
        assert!(rest.rest().is_empty());
    }

    #[test]
    fn expanding_puts_the_constructor_columns_in_place_of_the_first() {
        let fixture = QueryFixture::new();
        let constructor = Constructor::new(ConstructorTag::Bool(true), vec![QueryFixture::unit()]);
        let expanded = fixture
            .query()
            .expanded(&constructor, &[CoverPattern::wild()]);
        let query = expanded.query();
        assert_eq!(query.first_type(), &QueryFixture::unit());
        assert!(query.rest().first_pattern().head_tag() == Some(ConstructorTag::Bool(false)));
        assert_eq!(query.rest().first_type(), &QueryFixture::boolean());
    }
}
