use std::path::PathBuf;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::problems::LintProblems;
use super::read_back::ReadBack;
use super::rule_entries::RuleEntries;
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
    pub(super) const RULES: &str = "rule-names";
    const LOADER: &str = "lint-rules";

    pub fn expand_on(prelude: PreludeState) -> Result<Self, RuleFailure> {
        let (mut sources, loader) = Self::loader_sources();
        let expander = Expander::with_prelude(&mut sources, Self::module_sources(), prelude);
        Self::finish(
            expander.expand(ModulePath::new(Self::LOADER), loader),
            sources,
        )
    }

    pub fn expand_from_sources() -> Result<Self, RuleFailure> {
        let (mut sources, loader) = Self::loader_sources();
        let expander = Expander::new(&mut sources, Self::module_sources());
        Self::finish(
            expander.expand(ModulePath::new(Self::LOADER), loader),
            sources,
        )
    }

    fn loader_sources() -> (SourceMap, SourceId) {
        let mut sources = SourceMap::default();
        let uses = format!(
            "(use {} {} {})",
            ModuleSources::LINT,
            Self::CHECK,
            Self::RULES
        );
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
            check: Self::export_named(&program, Self::CHECK),
            names: Self::export_named(&program, Self::RULES),
        };
        Self {
            sources,
            program,
            definitions,
            entries,
        }
    }

    fn export_named(program: &Program, name: &str) -> BindingId {
        program
            .modules
            .iter()
            .filter(|module| module.path.as_str() == ModuleSources::LINT)
            .flat_map(|module| module.exports.iter().copied())
            .find(|binding| program.bindings.get(*binding).name() == name)
            .expect("the lint module exports its entries")
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
    use crate::lint::bound_names::BoundNames;
    use crate::lint::expanded_source::ExpandedSource;
    use crate::lint::rule_host::{ProgramFacts, RuleHost};
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
