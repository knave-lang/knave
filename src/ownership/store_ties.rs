use std::cell::RefCell;

use rustc_hash::{FxHashMap, FxHashSet};

use super::store_pairs::StorePairs;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, Lifetime, ParameterRole, ResultRegions, Type, TypeLifetimes, TypedDefinition,
    TypedExpr, TypedLambda, TypedProgram,
};

pub struct StoreTies<'p> {
    program: &'p TypedProgram,
    definitions: DefinitionTable<'p>,
    pairs_by_function: RefCell<FxHashMap<BindingId, StorePairs>>,
}

impl<'p> StoreTies<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            definitions: DefinitionTable::of(program),
            pairs_by_function: RefCell::default(),
        }
    }

    pub fn of(&self, callee: &TypedExpr) -> StorePairs {
        let Some(definition) = self.definitions.called_definition(callee) else {
            return StorePairs::default();
        };
        match definition.value.lambda_value() {
            Some(lambda) => self.remembered_pairs(definition, lambda),
            None => StorePairs::default(),
        }
    }

    fn remembered_pairs(&self, definition: &TypedDefinition, lambda: &TypedLambda) -> StorePairs {
        if let Some(known) = self.pairs_by_function.borrow().get(&definition.binding) {
            return known.clone();
        }
        let pairs = self.function_pairs(definition, lambda);
        self.pairs_by_function
            .borrow_mut()
            .insert(definition.binding, pairs.clone());
        pairs
    }

    fn function_pairs(&self, definition: &TypedDefinition, lambda: &TypedLambda) -> StorePairs {
        let Type::Function(_, declared, _) = &definition.value.ty else {
            return StorePairs::default();
        };
        let lifetimes = SignatureLifetimes {
            declared,
            shared: self.shared_positions(lambda),
        };
        lifetimes.store_pairs()
    }

    fn shared_positions(&self, lambda: &TypedLambda) -> FxHashSet<usize> {
        let regions = &self.program.ownership.regions;
        if regions.of(ParameterRole::StoredTarget, lambda).is_empty() {
            return FxHashSet::default();
        }
        let sharing = ResultRegions::of(regions, lambda).sharing();
        lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| sharing.contains(&parameter.binding))
            .map(|(position, _)| position)
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TieLifetime {
    Written(BindingId),
    Shared,
}

struct SignatureLifetimes<'d> {
    declared: &'d [Type],
    shared: FxHashSet<usize>,
}

impl SignatureLifetimes<'_> {
    fn store_pairs(&self) -> StorePairs {
        let mut pairs = StorePairs::default();
        for (target, referent) in self.targets() {
            for value in self.values_into(&referent) {
                pairs.tie(value, target);
            }
        }
        pairs
    }

    fn targets(&self) -> Vec<(usize, Vec<TieLifetime>)> {
        self.declared
            .iter()
            .enumerate()
            .filter_map(|(position, parameter)| match parameter {
                Type::Mut(_, referent) => Some((position, self.named(position, referent))),
                _ => None,
            })
            .collect()
    }

    fn values_into(&self, referent: &[TieLifetime]) -> Vec<usize> {
        self.declared
            .iter()
            .enumerate()
            .filter(|(_, parameter)| !matches!(parameter, Type::Mut(..)))
            .filter(|(position, parameter)| {
                self.named(*position, parameter)
                    .iter()
                    .any(|lifetime| referent.contains(lifetime))
            })
            .map(|(position, _)| position)
            .collect()
    }

    fn named(&self, position: usize, ty: &Type) -> Vec<TieLifetime> {
        TypeLifetimes::of(ty)
            .all()
            .into_iter()
            .filter_map(|lifetime| match lifetime {
                Lifetime::Written(binding) => Some(TieLifetime::Written(binding)),
                Lifetime::Elided if self.shared.contains(&position) => Some(TieLifetime::Shared),
                Lifetime::Elided => None,
            })
            .collect()
    }
}
