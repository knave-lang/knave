use super::bound_names::BoundNames;
use super::definition_reach::DefinitionReach;
use super::linted_module::LintedModule;
use super::module_table::ModuleTable;
use super::problems::LintProblems;
use super::read_back_definition::ReadBackDefinition;
use super::rule_failure::RuleFailure;
use super::rule_host::{ProgramFacts, RuleHost};
use super::rule_program::RuleProgram;
use super::rule_target::RuleTarget;
use super::spanned_forms::SpannedForms;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics};
use crate::expand::Program;
use crate::interpret::{StepBudget, Warnings};
use crate::source::{SourceMap, Span};

pub struct Linter<'p> {
    program: &'p Program,
    sources: &'p SourceMap,
    step_limit: u64,
}

impl<'p> Linter<'p> {
    pub fn new(program: &'p Program, sources: &'p SourceMap) -> Self {
        Self {
            program,
            sources,
            step_limit: StepBudget::DEFAULT_LIMIT,
        }
    }

    #[cfg(test)]
    pub fn with_step_limit(self, step_limit: u64) -> Self {
        Self { step_limit, ..self }
    }

    pub fn lint(&self, rules: &mut RuleProgram) -> Result<Diagnostics, RuleFailure> {
        let mut read_back = rules.read_back(&self.program.bindings);
        let modules: Vec<LintedModule<'_>> = self
            .program
            .project_modules()
            .map(|module| LintedModule::new(module, read_back.definitions(std::iter::once(module))))
            .collect();
        let names = read_back.into_names();
        let rules = &*rules;
        let mut host = RuleHost::new(rules, self.facts(&names));
        let reach = self.reach(rules, &mut host)?;
        let table = ModuleTable::new(self.program, self.sources, reach);
        for linted in modules {
            let target = table
                .target(&linted)
                .map_err(|failure| rules.failure(failure))?;
            for definition in linted.into_definitions() {
                self.check_definition(rules, &mut host, definition)?;
            }
            if let Some(target) = target {
                self.check_target(rules, &mut host, target)?;
            }
        }
        Ok(host.into_warnings())
    }

    fn reach(
        &self,
        rules: &RuleProgram,
        host: &mut RuleHost<'_>,
    ) -> Result<DefinitionReach, RuleFailure> {
        DefinitionReach::of(self.program, &mut |binding, span| {
            self.is_kept(host, binding, span)
        })
        .map_err(|failure| rules.failure(failure))
    }

    fn is_kept(
        &self,
        host: &mut RuleHost<'_>,
        binding: BindingId,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        let name = self.program.bindings.get(binding).name();
        match host.is_kept(SpannedForms::symbol(name, span), self.step_limit) {
            Err(failure) if Self::passed_budget(&failure) => Ok(true),
            answer => answer,
        }
    }

    fn facts<'n>(&'n self, names: &'n BoundNames) -> ProgramFacts<'n> {
        ProgramFacts {
            names,
            bindings: &self.program.bindings,
            origins: self.program.call_origins.as_ref(),
            sources: self.sources,
        }
    }

    fn check_definition(
        &self,
        rules: &RuleProgram,
        host: &mut RuleHost<'_>,
        definition: ReadBackDefinition,
    ) -> Result<(), RuleFailure> {
        let span = definition.form.span();
        let table = definition
            .table
            .into_value(span)
            .map_err(|deep| rules.failure(deep.at(span)))?;
        let target = RuleTarget::for_definition(table, definition.name, span);
        self.check_target(rules, host, target)
    }

    fn check_target(
        &self,
        rules: &RuleProgram,
        host: &mut RuleHost<'_>,
        target: RuleTarget,
    ) -> Result<(), RuleFailure> {
        let skipped = self
            .skipped_rules(rules, host, &target)
            .map_err(|failure| rules.failure(failure))?;
        if !skipped.is_empty() {
            let warning = LintProblems::rule_skipped(&skipped, target.name(), target.span());
            host.report_warning(warning);
        }
        Ok(())
    }

    fn skipped_rules(
        &self,
        rules: &RuleProgram,
        host: &mut RuleHost<'_>,
        target: &RuleTarget,
    ) -> Result<Vec<String>, Diagnostic> {
        let mut skipped = Vec::new();
        for rule in host.rule_names(target.span())? {
            let call = target.call(rule.clone(), rules.entries(), self.step_limit);
            match host.check_rule(call) {
                Ok(()) => {}
                Err(failure) if Self::passed_budget(&failure) => skipped.push(rule.to_string()),
                Err(failure) => return Err(failure),
            }
        }
        Ok(skipped)
    }

    fn passed_budget(failure: &Diagnostic) -> bool {
        failure.code() == Some(&DiagnosticCode::MACRO_STEP_LIMIT)
    }
}

#[cfg(test)]
mod tests {
    use crate::interpret::StepBudget;
    use crate::lint::expanded_source::ExpandedSource;

    struct LinterFixture;

    impl LinterFixture {
        const TINY_STEP_LIMIT: u64 = 10;
        const CHAIN_LENGTH: usize = 40;
        const CHAIN_STEP_LIMIT: u64 = 5_000;

        fn short_definition_and_long_chain() -> ExpandedSource {
            let locals: String = (1..=Self::CHAIN_LENGTH)
                .map(|index| format!(" (local v{index} (+ v{} 1))", index - 1))
                .collect();
            ExpandedSource::expand(&format!(
                "(export area chain)\n(defn area (width) (let (unused 1) width))\n\
                 (def chain (fn (v0){locals} v{}))",
                Self::CHAIN_LENGTH
            ))
        }
    }

    #[test]
    fn an_unread_local_is_reported() {
        let source =
            ExpandedSource::expand("(export area)\n(defn area (width) (let (unused 1) width))");
        assert_eq!(source.warning_titles(), vec!["`unused` is never read"]);
    }

    #[test]
    fn a_read_local_is_not_reported() {
        let source = ExpandedSource::expand(
            "(export area)\n(defn area (width) (let (used 1) (+ used width)))",
        );
        assert!(source.warning_titles().is_empty());
    }

    #[test]
    fn a_read_in_a_closure_counts() {
        let source = ExpandedSource::expand(
            "(export scaled)\n(defn scaled (numbers) (let (offset 1) (map (iter numbers) (fn (number) (* number offset)))))",
        );
        assert!(source.warning_titles().is_empty());
    }

    #[test]
    fn a_local_of_a_macro_is_not_reported() {
        let source = ExpandedSource::expand(
            "(export area)\n(defrule with-scratch ((body) (let (scratch 0) body)))\n(defn area (width) (with-scratch width))",
        );
        assert!(source.warning_titles().is_empty());
    }

    #[test]
    fn a_rule_past_its_budget_is_reported_as_skipped() {
        let source =
            ExpandedSource::expand("(export area)\n(defn area (width) (let (unused 1) width))");
        let titles = source.warning_titles_within(LinterFixture::TINY_STEP_LIMIT);
        assert_eq!(
            source.warning_codes_within(LinterFixture::TINY_STEP_LIMIT),
            vec!["L0805", "L0805"]
        );
        assert!(titles[0].ends_with("not checked in `area`"), "{titles:?}");
        assert!(
            titles[1].ends_with("not checked in `fixture`"),
            "{titles:?}"
        );
    }

    #[test]
    fn a_rule_skipped_on_one_definition_still_reports_on_another() {
        let source = LinterFixture::short_definition_and_long_chain();
        assert_eq!(
            source.warning_codes_within(LinterFixture::CHAIN_STEP_LIMIT),
            vec!["L0801", "L0805"]
        );
    }

    #[test]
    fn a_prelude_module_is_not_linted() {
        let source = ExpandedSource::expand("(defn main ())");
        let titles = source.warning_titles_within(LinterFixture::TINY_STEP_LIMIT);
        assert_eq!(
            source.warning_codes_within(LinterFixture::TINY_STEP_LIMIT),
            vec!["L0805", "L0805"]
        );
        assert!(titles[0].ends_with("in `main`"), "{titles:?}");
        assert!(titles[1].ends_with("in `fixture`"), "{titles:?}");
    }

    #[test]
    fn a_definition_whose_kept_test_passes_its_budget_is_kept() {
        let source =
            ExpandedSource::expand("(export area)\n(defn helper () 1)\n(defn area (width) width)");
        let unreachable = "L0807".to_string();
        assert!(
            source
                .warning_codes_within(StepBudget::DEFAULT_LIMIT)
                .contains(&unreachable)
        );
        assert!(
            !source
                .warning_codes_within(LinterFixture::TINY_STEP_LIMIT)
                .contains(&unreachable)
        );
    }
}
