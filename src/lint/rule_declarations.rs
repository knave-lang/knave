use super::library_declarations::LibraryDeclarations;
use super::trial_rule::TrialRule;
use super::written_program::WrittenProgram;
use crate::core::{BindingId, ModulePath};
use crate::expand::Program;

pub struct RuleDeclarations<'p, Rule: TrialRule> {
    rule: &'p Rule,
    program: &'p Program,
    written: WrittenProgram<'p>,
}

impl<'p, Rule: TrialRule> RuleDeclarations<'p, Rule> {
    pub fn new(rule: &'p Rule, program: &'p Program) -> Self {
        Self {
            rule,
            program,
            written: WrittenProgram::new(program),
        }
    }

    pub fn library_declarations(&self, chosen: &[BindingId]) -> Vec<LibraryDeclarations> {
        self.written
            .library_modules()
            .map(|module| LibraryDeclarations {
                module: module.path.clone(),
                names: self
                    .rule
                    .declared_in(self.program, module)
                    .into_iter()
                    .filter(|binding| chosen.contains(binding))
                    .map(|binding| self.written.name(binding))
                    .collect(),
            })
            .filter(|library| !library.names.is_empty())
            .collect()
    }

    pub fn named_in(&self, path: &ModulePath, names: &[String]) -> Vec<BindingId> {
        self.program
            .modules
            .iter()
            .filter(|module| &module.path == path)
            .flat_map(|module| self.rule.declared_in(self.program, module))
            .filter(|binding| names.contains(&self.written.name(*binding)))
            .collect()
    }
}
