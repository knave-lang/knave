use std::rc::Rc;

use super::Expander;
use super::problems::ExpandProblems;
use crate::core::{ModuleId, ModulePath, Namespace};
use crate::diagnostic::SimilarName;
use crate::interpret::{Interpreter, Value};
use crate::modules::ModuleSources;
use crate::reader::{Form, Symbol};
use crate::source::Span;

struct DocstringNotes {
    key: Rc<str>,
    documented_terms_key: Rc<str>,
    noted_term: Value,
}

impl DocstringNotes {
    fn of(key: &Value, documented_terms_key: &Value, noted_term: Value) -> Option<Self> {
        Some(Self {
            key: Self::symbol_name(key)?,
            documented_terms_key: Self::symbol_name(documented_terms_key)?,
            noted_term,
        })
    }

    fn symbol_name(value: &Value) -> Option<Rc<str>> {
        Some(value.as_form()?.as_symbol()?.shared_name())
    }
}

impl Expander<'_> {
    const PAGE_MACRO: &'static str = "doc-page";
    const DOCSTRINGS_KEY: &'static str = "docstrings-key";
    const DOCUMENTED_TERMS_KEY: &'static str = "documented-terms-key";
    const NOTED_TERM: &'static str = "noted-term";
    const TERM_NAMESPACES: [Namespace; 2] = [Namespace::Value, Namespace::Type];

    pub(super) fn check_documented_terms(&mut self, module: ModuleId) {
        if self.skips_document_check(module) {
            return;
        }
        let Some(notes) = self.docstring_notes() else {
            return;
        };
        let undefined: Vec<(Symbol, Span)> = self
            .documented_terms(module, &notes)
            .into_iter()
            .filter(|(term, _)| !self.defines(module, term))
            .filter(|(term, _)| !self.reported_as_duplicate(module, term))
            .collect();
        for (term, span) in undefined {
            let similar = self.closest_own_definition(module, &term);
            self.problems
                .report(ExpandProblems::undefined_documented_term(
                    term.name(),
                    span,
                    similar,
                ));
        }
    }

    fn skips_document_check(&self, module: ModuleId) -> bool {
        ModuleSources::is_prelude(self.modules.get(module).path())
            || !self.tables.module_notes.holds_any(module)
    }

    fn docstring_notes(&mut self) -> Option<DocstringNotes> {
        let notes_module = self.notes_module()?;
        let key = self.exported_value(notes_module, Self::DOCSTRINGS_KEY)?;
        let documented_terms_key = self.exported_value(notes_module, Self::DOCUMENTED_TERMS_KEY)?;
        let noted_term = self.exported_value(notes_module, Self::NOTED_TERM)?;
        DocstringNotes::of(&key, &documented_terms_key, noted_term)
    }

    fn notes_module(&self) -> Option<ModuleId> {
        let prelude = self
            .modules
            .find(&ModulePath::new(ModuleSources::PRELUDE))?;
        let page_macro = self
            .modules
            .get(prelude)
            .exported(Self::PAGE_MACRO, Namespace::Value)?;
        let page_module = self.definition_module(page_macro)?;
        let key = self
            .modules
            .get(page_module)
            .exported(Self::DOCSTRINGS_KEY, Namespace::Value)?;
        self.definition_module(key)
    }

    fn exported_value(&mut self, module: ModuleId, name: &str) -> Option<Value> {
        let binding = self.modules.get(module).exported(name, Namespace::Value)?;
        let span = self.tables.bindings.get(binding).span()?;
        self.evaluate_global(binding, span).ok()
    }

    fn documented_terms(
        &mut self,
        module: ModuleId,
        notes: &DocstringNotes,
    ) -> Vec<(Symbol, Span)> {
        [&notes.key, &notes.documented_terms_key]
            .into_iter()
            .flat_map(|key| self.tables.module_notes.of(module, key))
            .collect::<Vec<Form>>()
            .into_iter()
            .filter_map(|note| self.noted_term(&notes.noted_term, note))
            .collect()
    }

    fn noted_term(&mut self, accessor: &Value, note: Form) -> Option<(Symbol, Span)> {
        let span = note.span();
        let steps = self.form_budgets.current().steps();
        let term = Interpreter::new(self, span, steps)
            .call(accessor, vec![Value::Form(note)], span)
            .ok()?;
        let term = term.as_form()?;
        Some((term.as_symbol()?.clone(), term.span()))
    }

    fn reported_as_duplicate(&self, module: ModuleId, term: &Symbol) -> bool {
        self.duplicate_definitions
            .contains(&(module, term.shared_name()))
    }

    fn defines(&self, module: ModuleId, term: &Symbol) -> bool {
        Self::TERM_NAMESPACES
            .into_iter()
            .filter_map(|namespace| self.try_resolve(term, namespace))
            .any(|binding| self.definition_module(binding) == Some(module))
    }

    fn closest_own_definition(&self, module: ModuleId, term: &Symbol) -> Option<Rc<str>> {
        if self.resolves(term) {
            return None;
        }
        let own_names: Vec<Rc<str>> = Self::TERM_NAMESPACES
            .into_iter()
            .flat_map(|namespace| {
                self.tables
                    .table
                    .visible_name_bindings(term, namespace, self.phase)
            })
            .filter(|(_, binding)| self.definition_module(*binding) == Some(module))
            .map(|(name, _)| name)
            .collect();
        SimilarName::closest(term.name(), &own_names)
    }

    fn resolves(&self, term: &Symbol) -> bool {
        Self::TERM_NAMESPACES
            .into_iter()
            .any(|namespace| self.try_resolve(term, namespace).is_some())
    }
}
