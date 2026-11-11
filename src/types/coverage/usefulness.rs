use super::constructor::Constructor;
use super::constructor_catalog::ConstructorCatalog;
use super::constructor_tag::ConstructorTag;
use super::pattern_matrix::PatternMatrix;
use super::query::Query;
use super::witness::Witness;
use crate::core::BindingId;
use crate::types::Type;

pub(super) struct Usefulness<'c, 'a> {
    catalog: &'c ConstructorCatalog<'a>,
    expanding: Vec<BindingId>,
}

impl<'c, 'a> Usefulness<'c, 'a> {
    pub(super) fn new(catalog: &'c ConstructorCatalog<'a>) -> Self {
        Self {
            catalog,
            expanding: Vec::new(),
        }
    }

    pub(super) fn is_useful(&mut self, matrix: &PatternMatrix, query: Query) -> bool {
        if query.is_empty() {
            return matrix.is_empty();
        }
        if matrix.is_empty() {
            return true;
        }
        if matrix.has_wild_row() {
            return false;
        }
        let head = query.first_pattern();
        if head.is_wild() {
            return self.is_useful_wild(matrix, query);
        }
        match head.head_tag() {
            Some(tag @ ConstructorTag::Variant { .. }) => {
                self.is_useful_variant(matrix, query, tag)
            }
            Some(tag) => self.is_useful_tagged(matrix, query, tag),
            None => self.is_useful_literal(matrix, query),
        }
    }

    fn is_useful_wild(&mut self, matrix: &PatternMatrix, query: Query) -> bool {
        let owner = self.catalog.value_owner(query.first_type());
        if owner.is_some_and(|owner| self.expanding.contains(&owner)) {
            return self.is_useful_recursive_wild(matrix, query);
        }
        self.while_expanding(owner, |usefulness| {
            match usefulness.catalog.constructors(query.first_type()) {
                Some(constructors) => constructors.iter().any(|constructor| {
                    usefulness.is_useful_constructor(matrix, query, constructor)
                }),
                None => usefulness.is_useful(&matrix.default_rows(), query.rest()),
            }
        })
    }

    fn is_useful_recursive_wild(&mut self, matrix: &PatternMatrix, query: Query) -> bool {
        let default = matrix.default_rows();
        if !default.is_empty() && !self.is_useful(&default, query.rest()) {
            return false;
        }
        let concrete = matrix.concrete_rows();
        self.catalog
            .constructors(query.first_type())
            .unwrap_or_default()
            .iter()
            .any(|constructor| self.is_useful_constructor(&concrete, query, constructor))
    }

    fn is_useful_constructor(
        &mut self,
        matrix: &PatternMatrix,
        query: Query,
        constructor: &Constructor,
    ) -> bool {
        let expanded = query.expanded(constructor, &constructor.wild_arguments());
        self.is_useful_specialized(matrix, constructor, expanded.query())
    }

    fn is_useful_variant(
        &mut self,
        matrix: &PatternMatrix,
        query: Query,
        tag: ConstructorTag,
    ) -> bool {
        let owner = self
            .catalog
            .value_owner(query.first_type())
            .filter(|owner| !self.expanding.contains(owner));
        self.while_expanding(owner, |usefulness| {
            usefulness.is_useful_tagged(matrix, query, tag)
        })
    }

    fn is_useful_tagged(
        &mut self,
        matrix: &PatternMatrix,
        query: Query,
        tag: ConstructorTag,
    ) -> bool {
        let constructor = self
            .catalog
            .constructors(query.first_type())
            .and_then(|constructors| {
                constructors
                    .into_iter()
                    .find(|constructor| constructor.tag() == tag)
            });
        match constructor {
            Some(constructor) => {
                let expanded = query.expanded(&constructor, query.first_pattern().arguments());
                self.is_useful_specialized(matrix, &constructor, expanded.query())
            }
            None => false,
        }
    }

    fn is_useful_specialized(
        &mut self,
        matrix: &PatternMatrix,
        constructor: &Constructor,
        query: Query,
    ) -> bool {
        self.is_useful(&matrix.specialize(constructor), query)
    }

    fn is_useful_literal(&mut self, matrix: &PatternMatrix, query: Query) -> bool {
        let specialized = matrix.specialize_literal(query.first_pattern());
        self.is_useful(&specialized, query.rest())
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
