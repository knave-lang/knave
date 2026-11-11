use std::cell::Cell;

use super::constructor::Constructor;
use super::constructor_catalog::ConstructorCatalog;
use super::pattern_matrix::PatternMatrix;
use super::witness::Witness;
use crate::types::Type;

pub(super) struct MissingCaseSearch<'c, 'a> {
    catalog: &'c ConstructorCatalog<'a>,
    visited_matrices: Cell<usize>,
}

impl<'c, 'a> MissingCaseSearch<'c, 'a> {
    pub(super) fn new(catalog: &'c ConstructorCatalog<'a>) -> Self {
        Self {
            catalog,
            visited_matrices: Cell::new(0),
        }
    }

    pub(super) fn witnesses(&self, matrix: &PatternMatrix, types: &[Type]) -> Vec<Vec<Witness>> {
        self.visited_matrices.set(self.visited_matrices.get() + 1);
        self.settled_witnesses(matrix, types)
            .unwrap_or_else(|| self.column_witnesses(matrix, types))
    }

    #[cfg(test)]
    pub(super) fn visited_matrices(&self) -> usize {
        self.visited_matrices.get()
    }

    fn settled_witnesses(
        &self,
        matrix: &PatternMatrix,
        types: &[Type],
    ) -> Option<Vec<Vec<Witness>>> {
        if matrix.is_empty() {
            return Some(vec![vec![Witness::Wild; types.len()]]);
        }
        (types.is_empty() || matrix.has_wild_row()).then(Vec::new)
    }

    fn column_witnesses(&self, matrix: &PatternMatrix, types: &[Type]) -> Vec<Vec<Witness>> {
        let constructors = self.catalog.constructors(&types[0]).unwrap_or_default();
        let named =
            |constructor: &Constructor| matrix.heads_cover(std::slice::from_ref(constructor));
        if !constructors.iter().any(named) {
            return Self::prefixed(&Witness::Wild, &self.default_witnesses(matrix, types));
        }
        let defaults = match constructors.iter().all(named) {
            true => Vec::new(),
            false => self.default_witnesses(matrix, types),
        };
        constructors
            .iter()
            .flat_map(|constructor| match named(constructor) {
                true => self.constructor_witnesses(matrix, types, constructor),
                false => Self::prefixed(&Self::with_wild_arguments(constructor), &defaults),
            })
            .collect()
    }

    fn constructor_witnesses(
        &self,
        matrix: &PatternMatrix,
        types: &[Type],
        constructor: &Constructor,
    ) -> Vec<Vec<Witness>> {
        let columns = constructor.columns_with(&types[1..]);
        self.witnesses(&matrix.specialize(constructor), &columns)
            .into_iter()
            .map(|row| {
                let (arguments, rest) = row.split_at(constructor.arity());
                let mut witness = Vec::with_capacity(rest.len() + 1);
                witness.push(Witness::built_by(constructor, arguments));
                witness.extend_from_slice(rest);
                witness
            })
            .collect()
    }

    fn default_witnesses(&self, matrix: &PatternMatrix, types: &[Type]) -> Vec<Vec<Witness>> {
        self.witnesses(&matrix.default_rows(), &types[1..])
    }

    fn with_wild_arguments(constructor: &Constructor) -> Witness {
        Witness::built_by(constructor, &vec![Witness::Wild; constructor.arity()])
    }

    fn prefixed(head: &Witness, rests: &[Vec<Witness>]) -> Vec<Vec<Witness>> {
        rests
            .iter()
            .map(|rest| {
                let mut witness = Vec::with_capacity(rest.len() + 1);
                witness.push(head.clone());
                witness.extend_from_slice(rest);
                witness
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::coverage::cover_pattern::CoverPattern;
    use crate::types::coverage::search_fixture::SearchFixture;
    use crate::types::coverage::witness::Witness;

    #[test]
    fn an_exhaustive_match_misses_no_case() {
        let fixture = SearchFixture::new();
        let cases = fixture.missing_cases(SearchFixture::twin_arms(), SearchFixture::twin_type());
        assert!(cases.is_empty());
    }

    #[test]
    fn a_missing_variant_is_listed_as_its_case() {
        let fixture = SearchFixture::new();
        let mut arms = SearchFixture::chain_arms();
        arms.remove(0);
        let cases = fixture.missing_cases(arms, SearchFixture::chain_type());
        assert_eq!(cases.len(), 1);
        assert!(matches!(
            cases[0].as_slice(),
            [Witness::Variant { index: 1, arguments, .. }] if arguments.is_empty()
        ));
    }

    #[test]
    fn each_missing_case_is_one_value_of_the_scrutinee() {
        let fixture = SearchFixture::new();
        let wild = CoverPattern::wild;
        let arms = vec![SearchFixture::twin(SearchFixture::side(0), wild(), wild())];
        let cases = fixture.missing_cases(arms, SearchFixture::twin_type());
        assert!(!cases.is_empty());
        assert!(cases.iter().all(|row| row.len() == 1));
        assert!(
            cases
                .iter()
                .all(|row| matches!(row[0], Witness::Struct { .. }))
        );
    }

    #[test]
    fn fields_of_one_recursive_type_miss_one_case_together() {
        let fixture = SearchFixture::new();
        let cases = fixture.missing_cases(SearchFixture::wide_arms(), SearchFixture::wide_type());
        assert_eq!(cases.len(), 1);
        assert!(
            matches!(cases[0].as_slice(), [Witness::Struct { arguments, .. }] if arguments.len() == 8)
        );
    }

    #[test]
    fn a_match_without_arms_misses_every_value() {
        let fixture = SearchFixture::new();
        let cases = fixture.missing_cases(Vec::new(), SearchFixture::twin_type());
        assert_eq!(cases.len(), 1);
        assert!(matches!(cases[0].as_slice(), [Witness::Wild]));
    }

    #[test]
    fn a_row_of_wildcards_leaves_no_case() {
        let fixture = SearchFixture::new();
        let cases = fixture.missing_cases(vec![CoverPattern::wild()], SearchFixture::twin_type());
        assert!(cases.is_empty());
    }

    #[test]
    fn rows_without_columns_leave_no_case() {
        let fixture = SearchFixture::new();
        let cases = fixture.missing_cases_in(vec![SearchFixture::end()], &[]);
        assert!(cases.is_empty());
    }

    #[test]
    fn fields_of_one_type_are_searched_without_expanding_covered_rows() {
        let fixture = SearchFixture::new();
        let visited =
            fixture.visited_matrices(SearchFixture::wide_arms(), SearchFixture::wide_type());
        assert!(visited <= 100, "visited {visited} matrices");
    }
}
