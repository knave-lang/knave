use super::constructor::Constructor;
use super::constructor_catalog::ConstructorCatalog;
use super::pattern_matrix::PatternMatrix;
use super::witness::Witness;
use crate::core::BindingId;
use crate::types::Type;

pub(super) struct MissingCaseSearch<'c, 'a> {
    catalog: &'c ConstructorCatalog<'a>,
    expanding: Vec<BindingId>,
}

impl<'c, 'a> MissingCaseSearch<'c, 'a> {
    pub(super) fn new(catalog: &'c ConstructorCatalog<'a>) -> Self {
        Self {
            catalog,
            expanding: Vec::new(),
        }
    }

    pub(super) fn witnesses(
        &mut self,
        matrix: &PatternMatrix,
        types: &[Type],
    ) -> Vec<Vec<Witness>> {
        if types.is_empty() {
            return match matrix.is_empty() {
                true => vec![Vec::new()],
                false => Vec::new(),
            };
        }
        let (owner, constructors) = self.expansion(matrix, &types[0]);
        self.while_expanding(owner, |search| match constructors {
            Some(constructors) => constructors
                .iter()
                .flat_map(|constructor| search.constructor_witnesses(matrix, types, constructor))
                .collect(),
            None => search.wild_witnesses(matrix, types),
        })
    }

    fn constructor_witnesses(
        &mut self,
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

    fn wild_witnesses(&mut self, matrix: &PatternMatrix, types: &[Type]) -> Vec<Vec<Witness>> {
        self.witnesses(&matrix.default_rows(), &types[1..])
            .into_iter()
            .map(|row| {
                let mut witness = Vec::with_capacity(row.len() + 1);
                witness.push(Witness::Wild);
                witness.extend(row);
                witness
            })
            .collect()
    }

    fn expansion(
        &self,
        matrix: &PatternMatrix,
        ty: &Type,
    ) -> (Option<BindingId>, Option<Vec<Constructor>>) {
        match self.catalog.value_owner(ty) {
            Some(owner) if !self.expanding.contains(&owner) => {
                (Some(owner), self.catalog.constructors(ty))
            }
            Some(_) => (None, self.covered_constructors(matrix, ty)),
            None => (None, self.catalog.constructors(ty)),
        }
    }

    fn covered_constructors(&self, matrix: &PatternMatrix, ty: &Type) -> Option<Vec<Constructor>> {
        self.catalog
            .constructors(ty)
            .filter(|constructors| matrix.heads_cover(constructors))
    }

    fn while_expanding<T>(
        &mut self,
        owner: Option<BindingId>,
        work: impl FnOnce(&mut Self) -> T,
    ) -> T {
        if let Some(owner) = owner {
            self.expanding.push(owner);
        }
        let result = work(self);
        if owner.is_some() {
            self.expanding.pop();
        }
        result
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
        let cases = fixture.missing_cases(SearchFixture::twin_arms(), SearchFixture::TWIN);
        assert!(cases.is_empty());
    }

    #[test]
    fn a_missing_variant_is_listed_as_its_case() {
        let fixture = SearchFixture::new();
        let mut arms = SearchFixture::chain_arms();
        arms.remove(0);
        let cases = fixture.missing_cases(arms, SearchFixture::CHAIN);
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
        let cases = fixture.missing_cases(arms, SearchFixture::TWIN);
        assert!(!cases.is_empty());
        assert!(cases.iter().all(|row| row.len() == 1));
        assert!(
            cases
                .iter()
                .all(|row| matches!(row[0], Witness::Struct { .. }))
        );
    }
}
