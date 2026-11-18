use rustc_hash::FxHashSet;

use super::check_round::RoundGrowth;
use super::impl_bound_seeds::ImplBoundSeeds;
use super::typed::CheckWork;
use crate::source::Span;

#[derive(Default)]
pub(super) struct RoundFacts {
    vector_bases: FxHashSet<Span>,
    seeds: ImplBoundSeeds,
    checking: CheckWork,
}

impl RoundFacts {
    pub(super) fn vector_bases(&self) -> &FxHashSet<Span> {
        &self.vector_bases
    }

    pub(super) fn seeds(&self) -> &ImplBoundSeeds {
        &self.seeds
    }

    pub(super) fn checking(&self) -> CheckWork {
        self.checking
    }

    pub(super) fn grow(&mut self, growth: RoundGrowth, checking: CheckWork) {
        self.checking = checking;
        match growth {
            RoundGrowth::ImplBounds(seeds) => {
                debug_assert!(
                    seeds.strictly_extends(&self.seeds),
                    "each rerun adds a bound"
                );
                self.seeds = seeds;
            }
            RoundGrowth::VectorBases(found) => self.vector_bases.extend(found),
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::RoundFacts;
    use crate::source::{SourceId, Span};
    use crate::types::check_round::RoundGrowth;
    use crate::types::typed::CheckWork;

    struct RoundFactsFixture;

    impl RoundFactsFixture {
        fn base() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn grown_by_a_vector_base() -> RoundFacts {
            let mut facts = RoundFacts::default();
            let found: FxHashSet<Span> = [Self::base()].into_iter().collect();
            facts.grow(RoundGrowth::VectorBases(found), CheckWork::default());
            facts
        }
    }

    #[test]
    fn a_found_vector_base_joins_the_next_round() {
        let facts = RoundFactsFixture::grown_by_a_vector_base();
        assert!(facts.vector_bases().contains(&RoundFactsFixture::base()));
    }
}
