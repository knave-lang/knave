use rustc_hash::FxHashMap;

use super::bound_origins::BoundOrigin;
use super::ty::Bound;
use crate::core::BindingId;

#[derive(Clone, Default)]
pub struct TraitInclusions {
    included: FxHashMap<BindingId, BindingId>,
    chains: FxHashMap<BindingId, Vec<BindingId>>,
    including: FxHashMap<BindingId, Vec<BindingId>>,
}

impl TraitInclusions {
    pub fn add(&mut self, including: BindingId, included: BindingId) {
        self.included.insert(including, included);
        self.chains = self
            .included
            .keys()
            .map(|including| (*including, self.chain_from(*including)))
            .collect();
        self.including.clear();
        for (including, chain) in &self.chains {
            for included in chain {
                self.including
                    .entry(*included)
                    .or_default()
                    .push(*including);
            }
        }
        self.including
            .values_mut()
            .for_each(|including| including.sort());
    }

    pub fn included_traits(&self, including: BindingId) -> &[BindingId] {
        self.chains.get(&including).map_or(&[], Vec::as_slice)
    }

    pub fn including_traits(&self, included: BindingId) -> &[BindingId] {
        self.including.get(&included).map_or(&[], Vec::as_slice)
    }

    pub fn innermost(&self, binding: BindingId) -> BindingId {
        self.included_traits(binding)
            .last()
            .copied()
            .unwrap_or(binding)
    }

    pub fn generic_bound(&self, bound: &Bound) -> Option<Bound> {
        let innermost = self.innermost(bound.binding);
        (innermost != bound.binding).then(|| {
            let mut origins = bound.origins.clone();
            origins.add(BoundOrigin::Included(bound.binding));
            Bound {
                binding: innermost,
                argument: bound.argument.clone(),
                origins,
            }
        })
    }

    pub fn generic_bounds(&self, bounds: Vec<Bound>) -> Vec<Bound> {
        if self.included.is_empty() {
            return bounds;
        }
        let mut generic: Vec<Bound> = Vec::with_capacity(bounds.len());
        for bound in bounds {
            let bound = self.generic_bound(&bound).unwrap_or(bound);
            if !generic.contains(&bound) {
                generic.push(bound);
            }
        }
        generic
    }

    pub fn is_met_by(&self, required: BindingId, declared: BindingId) -> bool {
        required == declared || self.included_traits(required).contains(&declared)
    }

    fn chain_from(&self, including: BindingId) -> Vec<BindingId> {
        let mut chain = Vec::new();
        let mut current = including;
        while let Some(&next) = self.included.get(&current) {
            if next == including || chain.contains(&next) {
                break;
            }
            chain.push(next);
            current = next;
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::TraitInclusions;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};

    struct InclusionFixture {
        arena: BindingArena,
    }

    impl InclusionFixture {
        fn new() -> Self {
            Self {
                arena: BindingArena::default(),
            }
        }

        fn trait_binding(&mut self, name: &str) -> BindingId {
            self.arena
                .add(BindingInfo::new(name.into(), BindingKind::Local, None))
        }
    }

    #[test]
    fn a_chain_reaches_its_innermost_trait() {
        let mut fixture = InclusionFixture::new();
        let [outer, middle, inner] =
            ["outer", "middle", "inner"].map(|name| fixture.trait_binding(name));
        let mut inclusions = TraitInclusions::default();
        inclusions.add(outer, middle);
        inclusions.add(middle, inner);
        assert_eq!(inclusions.innermost(outer), inner);
        assert!(inclusions.is_met_by(outer, inner));
        assert_eq!(inclusions.including_traits(inner), [outer, middle]);
    }

    #[test]
    fn two_traits_may_include_one() {
        let mut fixture = InclusionFixture::new();
        let [first, second, shared] =
            ["first", "second", "shared"].map(|name| fixture.trait_binding(name));
        let mut inclusions = TraitInclusions::default();
        inclusions.add(first, shared);
        inclusions.add(second, shared);
        assert_eq!(inclusions.including_traits(shared), [first, second]);
    }

    #[test]
    fn a_cycle_ends_the_chain() {
        let mut fixture = InclusionFixture::new();
        let [left, right] = ["left", "right"].map(|name| fixture.trait_binding(name));
        let mut inclusions = TraitInclusions::default();
        inclusions.add(left, right);
        inclusions.add(right, left);
        assert_eq!(inclusions.included_traits(left), [right]);
        assert_eq!(inclusions.including_traits(left), [right]);
    }
}
