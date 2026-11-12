use super::bound_names::BoundNames;
use super::problems::LintProblems;
use super::read_back_definition::ReadBackDefinition;
use super::rule_call::RuleCall;
use super::rule_failure::RuleFailure;
use super::rule_host::{ProgramFacts, RuleHost};
use super::rule_program::RuleProgram;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics};
use crate::expand::Program;
use crate::interpret::{StepBudget, Value, Warnings};
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
        let definitions = read_back.definitions(self.program.project_modules());
        let names = read_back.into_names();
        let rules = &*rules;
        let mut host = RuleHost::new(rules, self.facts(&names));
        for definition in definitions {
            self.check_definition(rules, &mut host, definition)?;
        }
        Ok(host.into_warnings())
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
        let skipped = self
            .skipped_rules(host, &table, span)
            .map_err(|failure| rules.failure(failure))?;
        if !skipped.is_empty() {
            host.report_warning(LintProblems::rule_skipped(&skipped, &definition.name, span));
        }
        Ok(())
    }

    fn skipped_rules(
        &self,
        host: &mut RuleHost<'_>,
        table: &Value,
        span: Span,
    ) -> Result<Vec<String>, Diagnostic> {
        let mut skipped = Vec::new();
        for rule in host.rule_names(span)? {
            let call = RuleCall {
                rule: rule.clone(),
                table: table.clone(),
                span,
                step_limit: self.step_limit,
            };
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
                "(defn area (width) (let (unused 1) width))\n\
                 (def chain (fn (v0){locals} v{}))",
                Self::CHAIN_LENGTH
            ))
        }
    }

    #[test]
    fn an_unread_local_is_reported() {
        let source = ExpandedSource::expand("(defn area (width) (let (unused 1) width))");
        assert_eq!(source.warning_titles(), vec!["`unused` is never read"]);
    }

    #[test]
    fn a_read_local_is_not_reported() {
        let source = ExpandedSource::expand("(defn area (width) (let (used 1) (+ used width)))");
        assert!(source.warning_titles().is_empty());
    }

    #[test]
    fn a_read_in_a_closure_counts() {
        let source = ExpandedSource::expand(
            "(defn scaled (numbers) (let (offset 1) (map (iter numbers) (fn (number) (* number offset)))))",
        );
        assert!(source.warning_titles().is_empty());
    }

    #[test]
    fn a_local_of_a_macro_is_not_reported() {
        let source = ExpandedSource::expand(
            "(defrule with-scratch ((body) (let (scratch 0) body)))\n(defn area (width) (with-scratch width))",
        );
        assert!(source.warning_titles().is_empty());
    }

    #[test]
    fn a_rule_past_its_budget_is_reported_as_skipped() {
        let source = ExpandedSource::expand("(defn area (width) (let (unused 1) width))");
        assert_eq!(
            source.warning_titles_within(LinterFixture::TINY_STEP_LIMIT),
            vec!["`unused-binding` was not checked in `area`"]
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
        assert!(source.warning_titles().is_empty());
    }
}
