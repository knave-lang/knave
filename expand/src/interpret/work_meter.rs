use super::limit_problems::LimitProblems;
use super::step_budget::StepBudget;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

#[derive(Clone, Debug)]
pub struct WorkMeter {
    steps: StepBudget,
    call_site: Span,
}

impl WorkMeter {
    pub fn text_work(bytes: usize) -> usize {
        bytes.div_ceil(StepBudget::BYTES_PER_STEP)
    }

    pub fn built_forms_work(forms: usize) -> usize {
        forms.saturating_mul(StepBudget::STEPS_PER_BUILT_FORM)
    }

    pub fn new(steps: StepBudget, call_site: Span) -> Self {
        Self { steps, call_site }
    }

    pub fn call_site(&self) -> Span {
        self.call_site
    }

    #[inline]
    pub fn charge(&self, work: usize) -> Result<(), Diagnostic> {
        self.steps.spend_work(work as u64).map_err(|passed| {
            LimitProblems::too_many_steps(passed.limit, self.steps.first_passed_at(self.call_site))
        })
    }
}
