use rustc_hash::FxHashSet;

use super::store_pairs::StorePairs;
use crate::core::BindingId;
use crate::types::{
    Lifetime, SharedLifetime, Type, TypeLifetimes, TypedDefinition,
    TypedLambda, TypedProgram,
};

pub struct StoreTies<'p> {
    program: &'p TypedProgram,
}

impl<'p> StoreTies<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn of_function(&self, definition: &TypedDefinition, lambda: &TypedLambda) -> StorePairs {
        let Type::Function(_, declared, _) = &definition.value.ty else {
            return StorePairs::default();
        };
        let lifetimes = SignatureLifetimes {
            declared,
            shared: self.store_forced_positions(lambda),
        };
        lifetimes.store_pairs()
    }

    fn store_forced_positions(&self, lambda: &TypedLambda) -> FxHashSet<usize> {
        let shared_lifetime = SharedLifetime::of(&self.program.ownership.regions, lambda);
        if !shared_lifetime.forced_by_stores() {
            return FxHashSet::default();
        }
        let sharing = shared_lifetime.sharing();
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
