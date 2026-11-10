use std::cell::Cell;
use std::rc::Rc;

use crate::source::Span;

#[derive(Clone, Debug)]
pub struct StepBudget {
    counts: Rc<StepCounts>,
}

#[derive(Debug)]
struct StepCounts {
    limit: Cell<u64>,
    spent: Cell<u64>,
    first_passed_at: Cell<Option<Span>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassedStepLimit {
    pub limit: u64,
}

impl StepBudget {
    pub const DEFAULT_LIMIT: u64 = 30_000_000;
    const UNMETERED_LIMIT: u64 = u64::MAX;

    pub fn full() -> Self {
        Self::of(Self::DEFAULT_LIMIT)
    }

    pub fn unmetered() -> Self {
        Self::of(Self::UNMETERED_LIMIT)
    }

    fn of(limit: u64) -> Self {
        Self {
            counts: Rc::new(StepCounts {
                limit: Cell::new(limit),
                spent: Cell::new(0),
                first_passed_at: Cell::new(None),
            }),
        }
    }

    #[inline]
    pub fn spend(&self) -> Result<(), PassedStepLimit> {
        let spent = self.counts.spent.get();
        let limit = self.counts.limit.get();
        if spent < limit {
            self.counts.spent.set(spent + 1);
            Ok(())
        } else {
            Err(PassedStepLimit { limit })
        }
    }

    pub fn limit(&self) -> u64 {
        self.counts.limit.get()
    }

    pub fn first_passed_at(&self, call_site: Span) -> Span {
        let passed_at = self.counts.first_passed_at.get().unwrap_or(call_site);
        self.counts.first_passed_at.set(Some(passed_at));
        passed_at
    }

    pub fn set_limit(&self, limit: u64) {
        self.counts.limit.set(limit);
    }
}

#[cfg(test)]
mod tests {
    use super::{PassedStepLimit, StepBudget};
    use crate::source::{SourceId, Span};

    #[test]
    fn spends_exactly_the_limit() {
        let budget = StepBudget::of(2);
        assert!(budget.spend().is_ok());
        assert!(budget.spend().is_ok());
        assert_eq!(budget.spend(), Err(PassedStepLimit { limit: 2 }));
    }

    #[test]
    fn an_exhausted_budget_stays_exhausted() {
        let budget = StepBudget::of(0);
        assert!(budget.spend().is_err());
        assert!(budget.spend().is_err());
    }

    #[test]
    fn a_clone_spends_from_the_same_budget() {
        let budget = StepBudget::of(1);
        let clone = budget.clone();
        assert!(clone.spend().is_ok());
        assert!(budget.spend().is_err());
    }

    #[test]
    fn raising_the_limit_adds_the_difference() {
        let budget = StepBudget::of(1);
        budget.spend().expect("one step fits");
        budget.set_limit(3);
        assert!(budget.spend().is_ok());
        assert!(budget.spend().is_ok());
        assert_eq!(budget.spend(), Err(PassedStepLimit { limit: 3 }));
    }

    #[test]
    fn lowering_the_limit_below_the_spent_steps_exhausts_the_budget() {
        let budget = StepBudget::of(5);
        budget.spend().expect("one step fits");
        budget.spend().expect("two steps fit");
        budget.set_limit(1);
        assert_eq!(budget.spend(), Err(PassedStepLimit { limit: 1 }));
    }

    #[test]
    fn restoring_a_limit_keeps_the_spent_steps() {
        let budget = StepBudget::of(3);
        budget.spend().expect("one step fits");
        budget.set_limit(1);
        budget.set_limit(3);
        assert!(budget.spend().is_ok());
        assert!(budget.spend().is_ok());
        assert_eq!(budget.spend(), Err(PassedStepLimit { limit: 3 }));
    }

    #[test]
    fn the_first_call_that_passes_the_limit_keeps_the_blame() {
        let budget = StepBudget::of(0);
        let first = Span::new(SourceId::new(0), 0..1);
        let later = Span::new(SourceId::new(0), 2..3);
        assert_eq!(budget.first_passed_at(first), first);
        assert_eq!(budget.first_passed_at(later), first);
    }

    #[test]
    fn an_unmetered_budget_spends_without_end() {
        let budget = StepBudget::unmetered();
        for _ in 0..1_000 {
            assert!(budget.spend().is_ok());
        }
    }
}
