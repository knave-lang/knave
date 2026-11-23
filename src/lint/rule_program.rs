use std::path::PathBuf;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::problems::LintProblems;
use super::read_back::ReadBack;
use super::rule_entries::RuleEntries;
use super::rule_export::RuleExport;
use super::rule_failure::RuleFailure;
use super::rule_global::RuleGlobal;
use crate::core::{BindingArena, BindingId, BindingKind, Expr, ModulePath};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::expand::{Expander, PreludeState, Program};
use crate::modules::ModuleSources;
use crate::source::{SourceId, SourceMap, Span};

pub struct RuleProgram {
    sources: SourceMap,
    program: Program,
    definitions: FxHashMap<BindingId, Rc<Expr>>,
    entries: RuleEntries,
}

impl RuleProgram {
    const CHECK: &str = "check-rule";
    const CHECK_MODULE: &str = "check-module";
    const KEPT: &str = "kept-definition?";
    pub(super) const RULES: &str = "rule-names";
    const LOADER: &str = "lint-rules";

    pub fn expand_on(prelude: PreludeState) -> Result<Self, RuleFailure> {
        Self::expand_loading(&[], |sources, modules| {
            Expander::with_prelude(sources, modules, prelude)
        })
        .map(|(rules, _)| rules)
    }

    pub fn expand_from_sources() -> Result<Self, RuleFailure> {
        Self::expand_loading(&[], |sources, modules| Expander::new(sources, modules))
            .map(|(rules, _)| rules)
    }

    pub(super) fn expand_with_export(
        loaded: &str,
        export: RuleExport,
        expander: impl for<'s> FnOnce(&'s mut SourceMap, ModuleSources) -> Expander<'s>,
    ) -> Result<(Self, BindingId), RuleFailure> {
        let (rules, loader) = Self::expand_loading(&[loaded], expander)?;
        match Self::export_named(&rules.program, export.module, export.name) {
            Some(entry) => Ok((rules, entry)),
            None => {
                let missing = LintProblems::missing_export(
                    export.module,
                    export.name,
                    Span::new(loader, 0..0),
                );
                Err(rules.failure(missing))
            }
        }
    }

    fn expand_loading(
        modules: &[&str],
        expander: impl for<'s> FnOnce(&'s mut SourceMap, ModuleSources) -> Expander<'s>,
    ) -> Result<(Self, SourceId), RuleFailure> {
        let (mut sources, loader) = Self::loader_sources(modules);
        let expanded = expander(&mut sources, Self::module_sources())
            .expand(ModulePath::new(Self::LOADER), loader);
        Self::finish(expanded, sources).map(|rules| (rules, loader))
    }

    fn loader_sources(modules: &[&str]) -> (SourceMap, SourceId) {
        let mut sources = SourceMap::default();
        let mut uses = format!(
            "(use {} {} {} {} {})",
            ModuleSources::LINT,
            Self::CHECK,
            Self::CHECK_MODULE,
            Self::KEPT,
            Self::RULES
        );
        for module in modules {
            uses.push_str(&format!("\n(use {module})"));
        }
        let loader = sources.add(Self::LOADER, uses);
        (sources, loader)
    }

    fn module_sources() -> ModuleSources {
        ModuleSources::new(PathBuf::new(), PathBuf::new())
    }

    fn finish(
        expanded: Result<Program, Diagnostics>,
        sources: SourceMap,
    ) -> Result<Self, RuleFailure> {
        match expanded {
            Ok(program) => Ok(Self::of(sources, program)),
            Err(diagnostics) => Err(RuleFailure::of_all(&diagnostics, &sources)),
        }
    }

    fn of(sources: SourceMap, program: Program) -> Self {
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| module.definitions())
            .map(|(binding, value)| (binding, Rc::clone(value)))
            .collect();
        let entries = RuleEntries {
            check: Self::rule_entry(&program, Self::CHECK),
            check_module: Self::rule_entry(&program, Self::CHECK_MODULE),
            kept: Self::rule_entry(&program, Self::KEPT),
            names: Self::rule_entry(&program, Self::RULES),
        };
        Self {
            sources,
            program,
            definitions,
            entries,
        }
    }

    fn rule_entry(program: &Program, name: &str) -> BindingId {
        Self::export_named(program, ModuleSources::LINT, name)
            .expect("the lint module exports its entries")
    }

    fn export_named(program: &Program, module: &str, name: &str) -> Option<BindingId> {
        program
            .modules
            .iter()
            .filter(|candidate| candidate.path.as_str() == module)
            .find_map(|candidate| candidate.exports().binding_named(name, &program.bindings))
    }

    pub fn entries(&self) -> RuleEntries {
        self.entries
    }

    pub fn read_back<'p>(&'p mut self, bindings: &'p BindingArena) -> ReadBack<'p> {
        ReadBack::new(bindings, &mut self.program.scopes)
    }

    pub fn failure(&self, problem: Diagnostic) -> RuleFailure {
        RuleFailure::of(problem, &self.sources)
    }

    pub fn global(&self, binding: BindingId, span: Span) -> Result<RuleGlobal<'_>, Diagnostic> {
        if let BindingKind::CodeBuiltin(builtin) = self.program.bindings.get(binding).kind() {
            return Ok(RuleGlobal::Builtin(builtin));
        }
        self.definitions
            .get(&binding)
            .map(RuleGlobal::Definition)
            .ok_or_else(|| {
                LintProblems::outside_rules(self.program.bindings.get(binding).name(), span)
            })
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::RuleProgram;
    use crate::core::BindingArena;
    use crate::expand::Expander;
    use crate::lint::bound_names::BoundNames;
    use crate::lint::example_question::ExampleQuestion;
    use crate::lint::expanded_source::ExpandedSource;
    use crate::lint::rule_export::RuleExport;
    use crate::lint::rule_host::{ProgramFacts, RuleHost};
    use crate::modules::ModuleSources;
    use crate::reader::{Form, Symbol};

    struct ScopesFixture {
        rules: RuleProgram,
        source: ExpandedSource,
        arena: BindingArena,
    }

    impl ScopesFixture {
        fn new() -> Self {
            Self {
                rules: RuleProgram::expand_from_sources().expect("the lint rules expand"),
                source: ExpandedSource::expand("(def one 1)"),
                arena: BindingArena::default(),
            }
        }

        fn read_back_symbol(&mut self) -> Symbol {
            let program = self.source.program();
            let definitions = self
                .rules
                .read_back(&program.bindings)
                .definitions(program.modules.iter());
            let one = definitions
                .iter()
                .find(|definition| definition.name.as_ref() == "one")
                .expect("the source defines one");
            let parts = one.form.as_list().expect("a definition is a list");
            parts[1]
                .as_symbol()
                .expect("a definition names its binding")
                .clone()
        }

        fn rule_name_symbols(&self) -> Vec<Form> {
            let names = BoundNames::new(FxHashMap::default());
            let facts = ProgramFacts {
                names: &names,
                bindings: &self.arena,
                origins: self.source.program().call_origins.as_ref(),
                sources: self.source.sources(),
            };
            RuleHost::new(&self.rules, facts)
                .rule_names(self.source.span_of("(def one 1)"))
                .expect("the rule names evaluate")
        }
    }

    #[test]
    fn the_example_question_is_found_through_its_module_exports() {
        assert!(ExampleQuestion::expand_from_sources().is_ok());
    }

    #[test]
    fn a_missing_export_is_a_rule_failure() {
        let missing = RuleExport {
            module: ModuleSources::EXAMPLES,
            name: "no-such-question",
        };
        let expanded =
            RuleProgram::expand_with_export(ModuleSources::TEST, missing, |sources, modules| {
                Expander::new(sources, modules)
            });
        assert!(expanded.is_err());
    }

    #[test]
    fn a_read_back_scope_follows_every_scope_of_the_rule_program() {
        let mut fixture = ScopesFixture::new();
        let read_back = fixture.read_back_symbol();
        let rule_names = fixture.rule_name_symbols();
        let rule_scopes_reach_it = rule_names.iter().filter_map(Form::as_symbol).any(|name| {
            name.scopes()
                .any(|scope| read_back.scopes().any(|fresh| scope >= fresh))
        });
        let rule_names_carry_scopes = rule_names
            .iter()
            .filter_map(Form::as_symbol)
            .all(|name| name.scopes().any(|_| true));
        assert!(!rule_names.is_empty() && rule_names_carry_scopes);
        assert!(!rule_scopes_reach_it);
    }
}
