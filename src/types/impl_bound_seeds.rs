use rustc_hash::FxHashMap;

use super::impl_position::ImplPosition;
use super::ty::{Bound, GenericId, GenericParam};

struct SeededImpl {
    params: Vec<GenericId>,
    bounds: Vec<Vec<Bound>>,
}

#[derive(Default)]
pub(super) struct ImplBoundSeeds {
    by_position: FxHashMap<ImplPosition, SeededImpl>,
}

impl ImplBoundSeeds {
    pub(super) fn record(
        &mut self,
        position: ImplPosition,
        params: &[GenericParam],
        absorbed: &[Vec<Bound>],
    ) {
        if !absorbed
            .iter()
            .flatten()
            .any(|bound| bound.origins.is_needed_by_impl_body())
        {
            return;
        }
        let seeded = SeededImpl {
            params: Self::ids(params),
            bounds: absorbed.to_vec(),
        };
        self.by_position.insert(position, seeded);
    }

    pub(super) fn bounds_of(
        &self,
        position: ImplPosition,
        params: &[GenericParam],
    ) -> Option<Vec<Vec<Bound>>> {
        let seeded = self.by_position.get(&position)?;
        debug_assert_eq!(
            seeded.params,
            Self::ids(params),
            "every round collects the declarations in one order"
        );
        Some(seeded.bounds.clone())
    }

    pub(super) fn strictly_extends(&self, earlier: &ImplBoundSeeds) -> bool {
        let holds_earlier = earlier.by_position.iter().all(|(position, seeded)| {
            self.by_position
                .get(position)
                .is_some_and(|later| Self::holds_all(&later.bounds, &seeded.bounds))
        });
        holds_earlier && self.bound_count() > earlier.bound_count()
    }

    fn holds_all(later: &[Vec<Bound>], earlier: &[Vec<Bound>]) -> bool {
        later
            .iter()
            .zip(earlier)
            .all(|(later, earlier)| earlier.iter().all(|bound| later.contains(bound)))
    }

    fn bound_count(&self) -> usize {
        self.by_position
            .values()
            .flat_map(|seeded| &seeded.bounds)
            .map(Vec::len)
            .sum()
    }

    fn ids(params: &[GenericParam]) -> Vec<GenericId> {
        params.iter().map(|param| param.id).collect()
    }
}
