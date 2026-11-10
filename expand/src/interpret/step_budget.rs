use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct StepBudget {
    counts: Rc<StepCounts>,
}

#[derive(Debug)]
struct StepCounts {
    limit: Cell<u64>,
    remaining: Cell<u64>,
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
                remaining: Cell::new(limit),
            }),
        }
    }

    #[inline]
    pub fn spend(&self) -> Result<(), PassedStepLimit> {
        match self.counts.remaining.get().checked_sub(1) {
            Some(remaining) => {
                self.counts.remaining.set(remaining);
                Ok(())
            }
            None => Err(PassedStepLimit {
                limit: self.counts.limit.get(),
            }),
        }
    }

    pub fn set_limit(&self, limit: u64) {
        let spent = self.counts.limit.get() - self.counts.remaining.get();
        self.counts.limit.set(limit);
        self.counts.remaining.set(limit.saturating_sub(spent));
    }
}

#[cfg(test)]
mod tests {
    use super::{PassedStepLimit, StepBudget};

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
    fn an_unmetered_budget_spends_without_end() {
        let budget = StepBudget::unmetered();
        for _ in 0..1_000 {
            assert!(budget.spend().is_ok());
        }
    }
}
