use super::needless_type_search::NeedlessTypeSearch;
use super::program_run::ProgramRun;
use super::removal::Removal;
use super::trial_rule::TrialRule;
use crate::core::BindingId;
use crate::expand::Program;
use crate::types::TypedProgram;

pub struct RuleSearch<'r, Run: ProgramRun, Rule: TrialRule> {
    run: &'r mut Run,
    rule: &'r Rule,
}

impl<'r, Run: ProgramRun, Rule: TrialRule> RuleSearch<'r, Run, Rule> {
    pub fn new(run: &'r mut Run, rule: &'r Rule) -> Self {
        Self { run, rule }
    }

    pub fn jointly_removable(
        self,
        program: &mut Program,
        declared: &TypedProgram,
        tried: &[BindingId],
    ) -> Removal<Rule::Kept> {
        if tried.is_empty() {
            return Removal::default();
        }
        let mut trial = self.rule.trial(self.run, program, declared);
        NeedlessTypeSearch::new(&mut trial).jointly_removable(tried)
    }
}
