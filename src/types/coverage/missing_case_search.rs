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
        let (owner, constructors) = self.expansion(&types[0]);
        self.while_expanding(owner, |usefulness| match constructors {
            Some(constructors) => constructors
                .iter()
                .flat_map(|constructor| {
                    usefulness.constructor_witnesses(matrix, types, constructor)
                })
                .collect(),
            None => usefulness.wild_witnesses(matrix, types),
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

    fn expansion(&self, ty: &Type) -> (Option<BindingId>, Option<Vec<Constructor>>) {
        match self.catalog.value_owner(ty) {
            Some(owner) if !self.expanding.contains(&owner) => {
                (Some(owner), self.catalog.constructors(ty))
            }
            Some(_) => (None, None),
            None => (None, self.catalog.constructors(ty)),
        }
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
