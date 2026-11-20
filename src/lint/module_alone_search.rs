use super::checked_alone::CheckedAlone;
use super::library_declarations::LibraryDeclarations;
use super::program_position::ProgramPosition;
use super::program_run::ProgramRun;
use super::rule_declarations::RuleDeclarations;
use super::rule_search::RuleSearch;
use super::trial_rule::TrialRule;
use super::written_program::WrittenProgram;
use crate::core::{BindingId, ModulePath};
use crate::expand::Program;

pub struct ModuleAloneSearch<'r, Run: ProgramRun, Rule: TrialRule> {
    run: &'r mut Run,
    rule: &'r Rule,
    beside: ProgramPosition,
}

impl<'r, Run: ProgramRun, Rule: TrialRule> ModuleAloneSearch<'r, Run, Rule> {
    pub fn new(run: &'r mut Run, rule: &'r Rule, beside: ProgramPosition) -> Self {
        Self { run, rule, beside }
    }

    pub fn accepted(
        mut self,
        program: &Program,
        libraries: Vec<LibraryDeclarations>,
        removable: &[BindingId],
    ) -> Vec<BindingId> {
        let mut refused = Vec::new();
        let declarations = RuleDeclarations::new(self.rule, program);
        for library in libraries {
            let accepted = self.removable_names(&library);
            let refused_names: Vec<String> = library
                .names
                .into_iter()
                .filter(|name| !accepted.contains(name))
                .collect();
            refused.extend(declarations.named_in(&library.module, &refused_names));
        }
        removable
            .iter()
            .copied()
            .filter(|declared| !refused.contains(declared))
            .collect()
    }

    fn removable_names(&mut self, library: &LibraryDeclarations) -> Vec<String> {
        let Some(mut alone) = self.checked_alone(&library.module) else {
            return Vec::new();
        };
        let tried = RuleDeclarations::new(self.rule, &alone.program)
            .named_in(&library.module, &library.names);
        let removable = RuleSearch::new(&mut *self.run, self.rule)
            .jointly_removable(&mut alone.program, &alone.declared, &tried)
            .removable;
        let written = WrittenProgram::new(&alone.program);
        removable
            .into_iter()
            .map(|declared| written.name(declared))
            .collect()
    }

    fn checked_alone(&mut self, module: &ModulePath) -> Option<CheckedAlone> {
        let mut program = self.run.module_alone(self.beside, module)?;
        Self::check_every_definition_of(&mut program, module);
        let declared = self.run.check(&program).ok()?;
        Some(CheckedAlone { program, declared })
    }

    fn check_every_definition_of(alone: &mut Program, module: &ModulePath) {
        let defined: Vec<BindingId> = alone
            .modules
            .iter()
            .filter(|candidate| &candidate.path == module)
            .flat_map(|candidate| candidate.definitions().map(|(binding, _)| binding))
            .collect();
        for binding in defined {
            alone.check_also(binding);
        }
    }
}
