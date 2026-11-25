use rustc_hash::FxHashSet;

use super::store_pairs::StorePairs;
use super::stored_layer::StoredLayer;
use crate::core::BindingId;
use crate::types::{
    EmittedLifetimes, Lifetime, SharedLifetime, Type, TypeLifetimes, TypedDefinition, TypedLambda,
    TypedProgram,
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
            emitted: EmittedLifetimes::new(self.program),
            shared: self.store_forced_positions(lambda),
            elided_outer: self.store_only_positions(lambda),
        };
        lifetimes.store_pairs()
    }

    fn store_forced_positions(&self, lambda: &TypedLambda) -> FxHashSet<usize> {
        let shared_lifetime = self.shared_lifetime(lambda);
        if shared_lifetime.forced_by_stores() {
            Self::positions_of(lambda, &shared_lifetime.sharing())
        } else {
            FxHashSet::default()
        }
    }

    fn store_only_positions(&self, lambda: &TypedLambda) -> FxHashSet<usize> {
        Self::positions_of(lambda, &self.shared_lifetime(lambda).store_only_targets())
    }

    fn shared_lifetime<'l>(&self, lambda: &'l TypedLambda) -> SharedLifetime<'l, TypedLambda>
    where
        'p: 'l,
    {
        SharedLifetime::of(&self.program.ownership.regions, lambda)
    }

    fn positions_of(lambda: &TypedLambda, bindings: &FxHashSet<BindingId>) -> FxHashSet<usize> {
        lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| bindings.contains(&parameter.binding))
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
    emitted: EmittedLifetimes<'d>,
    shared: FxHashSet<usize>,
    elided_outer: FxHashSet<usize>,
}

impl SignatureLifetimes<'_> {
    fn store_pairs(&self) -> StorePairs {
        let mut pairs = StorePairs::default();
        for (target, referent) in self.targets() {
            for (value, layer) in self.values_into(target, &referent) {
                pairs.tie(value, layer, target);
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

    fn values_into(&self, target: usize, referent: &[TieLifetime]) -> Vec<(usize, StoredLayer)> {
        self.declared
            .iter()
            .enumerate()
            .filter(|(position, _)| *position != target)
            .filter_map(|(position, parameter)| {
                Some((position, self.stored_layer(position, parameter, referent)?))
            })
            .collect()
    }

    fn stored_layer(
        &self,
        position: usize,
        parameter: &Type,
        referent: &[TieLifetime],
    ) -> Option<StoredLayer> {
        let meets = |lifetimes: Vec<TieLifetime>| {
            lifetimes.iter().any(|lifetime| referent.contains(lifetime))
        };
        match parameter {
            Type::Ref(outer, inner) | Type::Mut(outer, inner) => {
                if meets(self.outer(position, *outer)) {
                    Some(StoredLayer::Whole)
                } else {
                    meets(self.named(position, inner)).then_some(StoredLayer::Referent)
                }
            }
            other => meets(self.named(position, other)).then_some(StoredLayer::Whole),
        }
    }

    fn outer(&self, position: usize, lifetime: Lifetime) -> Vec<TieLifetime> {
        if self.elided_outer.contains(&position) {
            return Vec::new();
        }
        self.tied(position, lifetime).into_iter().collect()
    }

    fn named(&self, position: usize, ty: &Type) -> Vec<TieLifetime> {
        TypeLifetimes::of(ty)
            .all_with(&self.emitted)
            .into_iter()
            .filter_map(|lifetime| self.tied(position, lifetime))
            .collect()
    }

    fn tied(&self, position: usize, lifetime: Lifetime) -> Option<TieLifetime> {
        match lifetime {
            Lifetime::Written(binding) => Some(TieLifetime::Written(binding)),
            Lifetime::Elided if self.shared.contains(&position) => Some(TieLifetime::Shared),
            Lifetime::Elided => None,
        }
    }
}
