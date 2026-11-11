use std::cell::Cell;
use std::rc::Rc;

use crate::source::Span;

#[derive(Clone, Debug)]
pub struct StepBudget {
    counts: Rc<StepCounts>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meter {
    On,
    Off,
}

#[derive(Debug)]
struct StepCounts {
    limit: Cell<u64>,
    spent: Cell<u64>,
    meter: Meter,
    first_passed_at: Cell<Option<Span>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassedStepLimit {
    pub limit: u64,
}

impl StepBudget {
    pub const DEFAULT_LIMIT: u64 = 30_000_000;
    pub const BYTES_PER_STEP: usize = 16;
    pub const EXPAND_CALL_WORK: usize = 64;
    pub const STEPS_PER_BUILT_FORM: usize = 3;
    const UNMETERED_LIMIT: u64 = u64::MAX;

    pub fn full() -> Self {
        Self::of(Self::DEFAULT_LIMIT)
    }

    pub fn unmetered() -> Self {
        Self::with_counts(Self::UNMETERED_LIMIT, Meter::Off)
    }

    fn of(limit: u64) -> Self {
        Self::with_counts(limit, Meter::On)
    }

    fn with_counts(limit: u64, meter: Meter) -> Self {
        Self {
            counts: Rc::new(StepCounts {
                limit: Cell::new(limit),
                spent: Cell::new(0),
                meter,
                first_passed_at: Cell::new(None),
            }),
        }
    }

    #[inline]
    pub fn spend(&self) -> Result<(), PassedStepLimit> {
        self.spend_work(1)
    }

    #[inline]
    pub fn spend_work(&self, work: u64) -> Result<(), PassedStepLimit> {
        let spent = self.counts.spent.get();
        let limit = self.counts.limit.get();
        if work <= limit.saturating_sub(spent) {
            self.counts.spent.set(spent + work);
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
        if self.counts.meter == Meter::On {
            self.counts.limit.set(limit);
        }
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
    fn work_spends_exactly_the_limit() {
        let budget = StepBudget::of(5);
        assert!(budget.spend_work(5).is_ok());
        assert_eq!(budget.spend_work(1), Err(PassedStepLimit { limit: 5 }));
    }

    #[test]
    fn work_past_the_rest_of_the_limit_is_refused_without_spending() {
        let budget = StepBudget::of(5);
        budget.spend_work(3).expect("three steps fit");
        assert!(budget.spend_work(3).is_err());
        assert!(budget.spend_work(2).is_ok());
    }

    #[test]
    fn work_at_the_largest_count_does_not_overflow() {
        let budget = StepBudget::of(5);
        assert!(budget.spend_work(u64::MAX).is_err());
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
    fn an_unmetered_budget_has_the_largest_limit() {
        assert_eq!(StepBudget::unmetered().limit(), u64::MAX);
    }

    #[test]
    fn an_unmetered_budget_spends_without_end() {
        let budget = StepBudget::unmetered();
        for _ in 0..1_000 {
            assert!(budget.spend().is_ok());
        }
    }

    #[test]
    fn setting_the_limit_of_an_unmetered_budget_does_nothing() {
        let budget = StepBudget::unmetered();
        budget.set_limit(0);
        assert_eq!(budget.limit(), u64::MAX);
        assert!(budget.spend().is_ok());
    }
}
