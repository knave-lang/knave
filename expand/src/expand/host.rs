use std::collections::BTreeSet;
use std::rc::Rc;

use super::Expander;
use super::expansion_limit::ExpansionLimit;
use super::failed_definition::DeclaredName;
use super::problems::ExpandProblems;
use crate::core::{BindingId, BindingKind, Declaration, ModuleDefinitions, ModulePath, Namespace};
use crate::diagnostic::Diagnostic;
use crate::interpret::{
    DefinitionQueries, ExpansionCallbacks, GlobalValues, Host, ModuleNotes, ModuleQueries,
    NameComparison, NotesTooLarge, SourceQueries, Value, Warnings,
};
use crate::modules::{ModuleRecord, ModuleSources};
use crate::reader::{Form, LiteralForms, Symbol};
use crate::scope::{Phase, Resolution, ScopeSet};
use crate::source::Span;

impl Host for Expander<'_> {
    fn global_values(&mut self) -> Option<&mut dyn GlobalValues> {
        Some(self)
    }

    fn module_queries(&mut self) -> Option<&mut dyn ModuleQueries> {
        Some(self)
    }

    fn definition_queries(&mut self) -> Option<&mut dyn DefinitionQueries> {
        Some(self)
    }

    fn module_notes(&mut self) -> Option<&mut dyn ModuleNotes> {
        Some(self)
    }

    fn name_comparison(&mut self) -> Option<&mut dyn NameComparison> {
        Some(self)
    }

    fn expansion_callbacks(&mut self) -> Option<&mut dyn ExpansionCallbacks> {
        Some(self)
    }

    fn source_queries(&mut self) -> Option<&mut dyn SourceQueries> {
        Some(self)
    }

    fn warnings(&mut self) -> Option<&mut dyn Warnings> {
        Some(self)
    }
}

impl GlobalValues for Expander<'_> {
    fn global_value(&mut self, binding: BindingId, span: Span) -> Result<Value, Diagnostic> {
        match self.kind(binding) {
            BindingKind::CodeBuiltin(builtin) => Ok(Value::Builtin(builtin)),
            BindingKind::Global(_) => self.evaluate_global(binding, span),
            _ => Err(ExpandProblems::not_at_compile_time(
                self.tables.bindings.get(binding).name(),
                span,
            )),
        }
    }
}

impl Warnings for Expander<'_> {
    fn report_warning(&mut self, warning: Diagnostic) {
        self.problems.report_warning(warning);
    }
}

impl ModuleQueries for Expander<'_> {
    fn program_modules(&self) -> Vec<Symbol> {
        self.modules
            .records()
            .filter(|record| ModuleSources::is_written(record.path()))
            .map(ModuleRecord::symbol)
            .collect()
    }

    fn project_modules(&self) -> Vec<Symbol> {
        self.modules
            .records()
            .filter(|record| ModuleSources::is_written(record.path()))
            .filter(|record| !self.is_library_module(record.path()))
            .map(ModuleRecord::symbol)
            .collect()
    }

    fn module_exports(&self, path: &Symbol, span: Span) -> Result<Vec<Symbol>, Diagnostic> {
        let record = self.loaded_module(path, span)?;
        let names: BTreeSet<&str> = record.exports().map(|(name, _, _)| &**name).collect();
        Ok(names
            .into_iter()
            .map(|name| Symbol::with_scopes(name, record.scopes().set()))
            .collect())
    }

    fn module_definitions(&self, path: &Symbol, span: Span) -> Result<Vec<Symbol>, Diagnostic> {
        let record = self.loaded_module(path, span)?;
        let names = self
            .modules
            .find(record.path())
            .map_or_else(Rc::default, |module| {
                ModuleDefinitions::sorted_names(&self.tables.bindings, module)
            });
        let scopes = record.scopes().set();
        Ok(names
            .iter()
            .map(|name| Symbol::with_scopes(Rc::clone(name), scopes.clone()))
            .collect())
    }

    fn current_module(&self) -> Symbol {
        let module = self
            .current_module
            .expect("macro code runs while a module expands");
        self.modules.get(module).symbol()
    }

    fn module_symbol(&self, path: &Symbol, span: Span) -> Result<Symbol, Diagnostic> {
        let record = self.loaded_module(path, span)?;
        if ModuleSources::is_compiler_provided(record.path()) {
            return Err(ExpandProblems::unknown_module(record.path(), span));
        }
        Ok(record.symbol())
    }
}

impl DefinitionQueries for Expander<'_> {
    fn definition_symbol(
        &self,
        term: &Symbol,
        name: String,
        span: Span,
    ) -> Result<Symbol, Diagnostic> {
        let binding = self.bound_term(term, span)?;
        Ok(Symbol::with_scopes(name, self.definition_scopes(binding)))
    }

    fn type_definition_symbol(&self, term: &Symbol, name: String) -> Symbol {
        let scopes = self
            .namespace_binding(term, Namespace::Type)
            .map_or_else(ScopeSet::default, |binding| self.definition_scopes(binding));
        Symbol::with_scopes(name, scopes)
    }

    fn definition_call(&self, term: &Symbol, span: Span) -> Result<Option<Form>, Diagnostic> {
        let binding = self.bound_term(term, span)?;
        Ok(self.defining_call(binding))
    }

    fn type_definition_call(&self, term: &Symbol) -> Option<Form> {
        let binding = self.namespace_binding(term, Namespace::Type)?;
        self.defining_call(binding)
    }
}

impl ModuleNotes for Expander<'_> {
    fn add_module_note(&mut self, key: &str, note: Form) -> Result<(), NotesTooLarge> {
        let module = self
            .current_module
            .expect("macro code runs while a module expands");
        self.tables.module_notes.add(module, key, note)
    }

    fn module_notes(&self, context: &Symbol, key: &str) -> Vec<Form> {
        self.modules
            .with_body_scopes(context.scopes())
            .map_or_else(Vec::new, |module| self.tables.module_notes.of(module, key))
    }
}

impl NameComparison for Expander<'_> {
    fn same_binding(&self, left: &Symbol, right: &Symbol) -> bool {
        let pairs: Vec<(Resolution, Resolution)> = [Namespace::Value, Namespace::Type]
            .into_iter()
            .map(|namespace| {
                (
                    self.tables.table.resolve(left, namespace, Phase::Runtime),
                    self.tables.table.resolve(right, namespace, Phase::Runtime),
                )
            })
            .collect();
        let bound_together = pairs.iter().any(|pair| {
            matches!(pair, (Resolution::Bound(left_binding), Resolution::Bound(right_binding))
                if left_binding == right_binding)
        });
        let unbound_everywhere = pairs
            .iter()
            .all(|pair| matches!(pair, (Resolution::Unbound, Resolution::Unbound)));
        bound_together || (unbound_everywhere && left.name() == right.name())
    }

    fn is_pattern_head(&self, head: &Symbol) -> bool {
        self.names_pattern_head(head)
    }
}

impl ExpansionCallbacks for Expander<'_> {
    fn expand_call(&mut self, call: Form) -> Result<Option<Form>, Diagnostic> {
        let Some(binding) = self.macro_head(&call) else {
            return Ok(None);
        };
        match self.discarded_expansion(binding, call) {
            Ok(expansion) => Ok(Some(expansion)),
            Err(failure) if ExpansionLimit::stopped(&failure) => Err(failure),
            Err(_) => Ok(None),
        }
    }

    fn set_macro_step_limit(&mut self, limit: u64) {
        self.form_budgets.current().steps().set_limit(limit);
    }

    fn declare_definition(&mut self, name: Form, declaration: Declaration) {
        let introduction = *self
            .introductions
            .last()
            .expect("macro code runs inside a macro call");
        self.declared_definitions
            .push(DeclaredName::new(name, declaration, introduction));
    }

    fn require_module(&mut self, path: &Symbol, span: Span) -> Result<Symbol, Diagnostic> {
        let path = ModulePath::new(path.name());
        if !ModuleSources::is_embedded_prelude(&path) || ModuleSources::is_internal(&path) {
            return Err(ExpandProblems::unknown_module(&path, span));
        }
        let module = self.load_module(&path, span)?;
        Ok(self.modules.get(module).symbol())
    }
}

impl SourceQueries for Expander<'_> {
    fn form_location(&self, form: &Form) -> String {
        let span = form.span();
        let file = self.sources.file(span.source());
        let position = self.sources.start_position(span);
        format!(
            "{}:{position}",
            self.module_sources.relative_name(file.name())
        )
    }

    fn in_tasks_directory(&self, form: &Form) -> bool {
        let file = self.sources.file(form.span().source());
        self.module_sources.holds_task(file.name())
    }

    fn introducing_macro(&self, form: &Form) -> Option<Rc<str>> {
        let origin = self.tables.macro_origins.get(form.span())?;
        Some(origin.name)
    }

    fn read_literal_form(
        &self,
        literal: Span,
        value: &str,
        character: usize,
    ) -> Option<(Form, usize)> {
        LiteralForms::new(self.sources.file(literal.source()), literal.range(), value)
            .read_at(character)
    }
}

impl Expander<'_> {
    fn loaded_module(&self, path: &Symbol, span: Span) -> Result<&ModuleRecord, Diagnostic> {
        let path = ModulePath::new(path.name());
        let module = self
            .modules
            .find(&path)
            .ok_or_else(|| ExpandProblems::unknown_module(&path, span))?;
        Ok(self.modules.get(module))
    }

    fn definition_scopes(&self, binding: BindingId) -> ScopeSet {
        if let Some(module) = self.definition_module(binding) {
            return self.modules.get(module).scopes().set();
        }
        match self.kind(binding) {
            BindingKind::KernelForm(_) | BindingKind::KernelType(_) | BindingKind::Boolean(_) => {
                self.prelude_scopes()
            }
            _ => ScopeSet::default(),
        }
    }

    fn prelude_scopes(&self) -> ScopeSet {
        let prelude = self
            .modules
            .find(&ModulePath::new(ModuleSources::PRELUDE))
            .expect("macro code runs after the prelude module is registered");
        self.modules.get(prelude).scopes().set()
    }

    fn bound_term(&self, term: &Symbol, span: Span) -> Result<BindingId, Diagnostic> {
        self.term_binding(term)
            .ok_or_else(|| self.unbound(term, Namespace::Value, span))
    }

    pub(super) fn term_binding(&self, term: &Symbol) -> Option<BindingId> {
        [Namespace::Value, Namespace::Type]
            .into_iter()
            .find_map(|namespace| self.namespace_binding(term, namespace))
    }

    pub(super) fn namespace_binding(
        &self,
        term: &Symbol,
        namespace: Namespace,
    ) -> Option<BindingId> {
        match self.tables.table.resolve(term, namespace, Phase::Runtime) {
            Resolution::Bound(binding) => Some(binding),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::Expander;
    use crate::core::{BindingInfo, BindingKind, Builtin};
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::expand::macro_call::MacroOrigin;
    use crate::interpret::{GlobalValues, Host, Interpreter, SourceQueries, StepBudget, Value};
    use crate::modules::ModuleSources;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceMap, Span};

    struct HostFixture {
        sources: SourceMap,
    }

    impl HostFixture {
        fn new() -> Self {
            Self {
                sources: SourceMap::default(),
            }
        }

        fn expander(&mut self) -> Expander<'_> {
            Expander::new(
                &mut self.sources,
                ModuleSources::new(PathBuf::new(), PathBuf::new()),
            )
        }

        fn global_value_of(&mut self, kind: BindingKind, span: Span) -> Result<(), Diagnostic> {
            let mut expander = self.expander();
            let binding =
                expander
                    .tables
                    .bindings
                    .add(BindingInfo::new("origin".into(), kind, None));
            expander.global_value(binding, span).map(drop)
        }

        fn span(&mut self) -> Span {
            let source = self.sources.add("sample.knv", "origin");
            Span::new(source, 0..6)
        }

        fn introducing_macro_at(&mut self, call_site: Span, binder: Span) -> Option<Rc<str>> {
            let expander = self.expander_with_origin_at(call_site);
            expander.introducing_macro(&Self::hidden(binder))
        }

        fn called_introducing_macro_at(&mut self, call_site: Span) -> Option<String> {
            let builtin = Builtin::all().find(|builtin| builtin.name() == "introducing-macro")?;
            let mut expander = self.expander_with_origin_at(call_site);
            let hidden = Value::Form(Self::hidden(call_site));
            let names = Interpreter::new(&mut expander, call_site, StepBudget::full())
                .call(&Value::Builtin(builtin), vec![hidden], call_site)
                .ok()?;
            names.as_form().map(Form::to_string)
        }

        fn expander_with_origin_at(&mut self, call_site: Span) -> Expander<'_> {
            let mut expander = self.expander();
            let origin = MacroOrigin {
                name: "hide-a-name".into(),
                defined: call_site,
                call_site,
                through_prelude: false,
            };
            expander.tables.macro_origins.record(call_site, origin);
            expander
        }

        fn hidden(binder: Span) -> Form {
            Form::new(FormKind::Symbol(Symbol::new("hidden")), binder)
        }
    }

    #[test]
    fn the_introducing_macro_built_in_names_the_macro_at_its_call() {
        let mut fixture = HostFixture::new();
        let call_site = fixture.span();
        assert_eq!(
            fixture.called_introducing_macro_at(call_site).as_deref(),
            Some("(hide-a-name)")
        );
    }

    #[test]
    fn a_form_at_a_macro_call_names_the_macro_that_introduced_it() {
        let mut fixture = HostFixture::new();
        let call_site = fixture.span();
        let name = fixture.introducing_macro_at(call_site, call_site);
        assert_eq!(name.as_deref(), Some("hide-a-name"));
    }

    #[test]
    fn a_form_written_inside_a_macro_call_names_no_macro() {
        let mut fixture = HostFixture::new();
        let call_site = fixture.span();
        let written = Span::new(call_site.source(), 1..4);
        assert_eq!(fixture.introducing_macro_at(call_site, written), None);
    }

    #[test]
    fn macro_code_cannot_read_a_binding_that_is_no_global() {
        let mut fixture = HostFixture::new();
        let span = fixture.span();
        let refusal = fixture
            .global_value_of(BindingKind::TypeVariable, span)
            .expect_err("a type variable has no value");
        assert_eq!(refusal.code(), Some(&DiagnosticCode::NOT_AT_COMPILE_TIME));
        let spans: Vec<Span> = refusal.labels().map(|label| label.span()).collect();
        assert_eq!(spans, vec![span]);
    }

    #[test]
    fn the_expander_offers_every_host_group() {
        let mut fixture = HostFixture::new();
        let mut expander = fixture.expander();
        assert!(expander.global_values().is_some(), "global values");
        assert!(expander.module_queries().is_some(), "module queries");
        assert!(
            expander.definition_queries().is_some(),
            "definition queries"
        );
        assert!(expander.module_notes().is_some(), "module notes");
        assert!(expander.name_comparison().is_some(), "name comparison");
        assert!(
            expander.expansion_callbacks().is_some(),
            "expansion callbacks"
        );
        assert!(expander.source_queries().is_some(), "source queries");
        assert!(expander.warnings().is_some(), "warnings");
    }
}
