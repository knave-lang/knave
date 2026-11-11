use super::column_head::ColumnHead;
use super::constructor::Constructor;
use super::constructor_catalog::ConstructorCatalog;
use super::constructor_tag::ConstructorTag;
use super::pattern_matrix::PatternMatrix;
use super::query::Query;
use crate::core::Literal;

pub(super) struct Usefulness<'c, 'a> {
    catalog: &'c ConstructorCatalog<'a>,
}

impl<'c, 'a> Usefulness<'c, 'a> {
    pub(super) fn new(catalog: &'c ConstructorCatalog<'a>) -> Self {
        Self { catalog }
    }

    pub(super) fn is_useful(&self, matrix: &PatternMatrix, query: Query) -> bool {
        if query.is_empty() {
            return matrix.is_empty();
        }
        if matrix.is_empty() {
            return true;
        }
        if matrix.has_wild_row() {
            return false;
        }
        match query.first_pattern().kind().head() {
            ColumnHead::Any => self.is_useful_wild(matrix, query),
            ColumnHead::Constructor(tag) => self.is_useful_tagged(matrix, query, tag),
            ColumnHead::Literal(literal) => self.is_useful_literal(matrix, query, literal),
        }
    }

    fn is_useful_wild(&self, matrix: &PatternMatrix, query: Query) -> bool {
        match self
            .catalog
            .constructors_named_by(query.first_type(), matrix)
        {
            Some(constructors) => constructors
                .iter()
                .any(|constructor| self.is_useful_constructor(matrix, query, constructor)),
            None => self.is_useful(&matrix.default_rows(), query.rest()),
        }
    }

    fn is_useful_tagged(&self, matrix: &PatternMatrix, query: Query, tag: ConstructorTag) -> bool {
        self.catalog
            .constructors(query.first_type())
            .and_then(|constructors| {
                constructors
                    .into_iter()
                    .find(|constructor| constructor.tag() == tag)
            })
            .is_some_and(|constructor| self.is_useful_constructor(matrix, query, &constructor))
    }

    fn is_useful_constructor(
        &self,
        matrix: &PatternMatrix,
        query: Query,
        constructor: &Constructor,
    ) -> bool {
        let arguments = constructor
            .arguments_of(query.first_pattern())
            .expect("a query's first pattern fits each constructor it is searched under");
        let expanded = query.expanded(constructor, &arguments);
        self.is_useful(&matrix.specialize(constructor), expanded.query())
    }

    fn is_useful_literal(&self, matrix: &PatternMatrix, query: Query, literal: &Literal) -> bool {
        self.is_useful(&matrix.specialize_literal(literal), query.rest())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::coverage::cover_pattern::CoverPattern;
    use crate::types::coverage::search_fixture::SearchFixture;

    #[test]
    fn columns_of_one_type_are_searched_apart() {
        let fixture = SearchFixture::new();
        let exhaustive = fixture.is_useful(
            SearchFixture::twin_arms(),
            CoverPattern::wild(),
            SearchFixture::TWIN,
        );
        assert!(!exhaustive);
    }

    #[test]
    fn a_nested_recursive_enum_keeps_its_wildcard_rows() {
        let fixture = SearchFixture::new();
        let exhaustive = fixture.is_useful(
            SearchFixture::chain_arms(),
            CoverPattern::wild(),
            SearchFixture::CHAIN,
        );
        assert!(!exhaustive);
    }

    #[test]
    fn a_missing_case_of_a_recursive_enum_is_useful() {
        let fixture = SearchFixture::new();
        let mut arms = SearchFixture::chain_arms();
        arms.remove(2);
        assert!(fixture.is_useful(arms, CoverPattern::wild(), SearchFixture::CHAIN));
    }

    #[test]
    fn a_missing_case_beside_a_column_of_the_same_type_is_useful() {
        let fixture = SearchFixture::new();
        let mut arms = SearchFixture::twin_arms();
        arms.remove(3);
        assert!(fixture.is_useful(arms, CoverPattern::wild(), SearchFixture::TWIN));
    }

    #[test]
    fn an_arm_that_earlier_arms_cover_together_is_not_useful() {
        let fixture = SearchFixture::new();
        let wild = CoverPattern::wild;
        let covered =
            SearchFixture::twin(SearchFixture::side(1), wild(), SearchFixture::flag(true));
        assert!(!fixture.is_useful(SearchFixture::twin_arms(), covered, SearchFixture::TWIN));
    }

    #[test]
    fn an_arm_with_a_case_no_earlier_arm_covers_is_useful() {
        let fixture = SearchFixture::new();
        let wild = CoverPattern::wild;
        let arms = vec![SearchFixture::twin(SearchFixture::side(0), wild(), wild())];
        let new_case = SearchFixture::twin(SearchFixture::side(1), wild(), wild());
        assert!(fixture.is_useful(arms, new_case, SearchFixture::TWIN));
    }
}
