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
    fn an_unmetered_budget_spends_without_end() {
        let budget = StepBudget::unmetered();
        for _ in 0..1_000 {
            assert!(budget.spend().is_ok());
        }
    }
}
