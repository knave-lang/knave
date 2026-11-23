use super::rule_export::RuleExport;
use super::rule_failure::RuleFailure;
use super::rule_program::RuleProgram;
use crate::core::BindingId;
use crate::expand::{Expander, PreludeState};
use crate::modules::ModuleSources;

pub struct ExampleQuestion {
    rules: RuleProgram,
    entry: BindingId,
}

impl ExampleQuestion {
    const QUESTION: RuleExport = RuleExport {
        module: ModuleSources::EXAMPLES,
        name: "notes-hold-example?",
    };

    pub fn expand_on(prelude: PreludeState) -> Result<Self, RuleFailure> {
        RuleProgram::expand_with_export(ModuleSources::TEST, Self::QUESTION, |sources, modules| {
            Expander::with_prelude(sources, modules, prelude)
        })
        .map(Self::asking)
    }

    pub fn expand_from_sources() -> Result<Self, RuleFailure> {
        RuleProgram::expand_with_export(ModuleSources::TEST, Self::QUESTION, |sources, modules| {
            Expander::new(sources, modules)
        })
        .map(Self::asking)
    }

    pub fn rules(&mut self) -> &mut RuleProgram {
        &mut self.rules
    }

    pub fn entry(&self) -> BindingId {
        self.entry
    }

    fn asking((rules, entry): (RuleProgram, BindingId)) -> Self {
        Self { rules, entry }
    }
}
