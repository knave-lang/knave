use std::rc::Rc;

use super::Expander;
use super::problems::ExpandProblems;
use crate::core::{ModuleId, ModulePath, Namespace};
use crate::interpret::{Interpreter, Value};
use crate::modules::ModuleSources;
use crate::reader::{Form, Symbol};
use crate::source::Span;

struct DocstringNotes {
    key: Rc<str>,
    noted_term: Value,
}

impl Expander<'_> {
    const PAGE_MACRO: &'static str = "doc-page";
    const DOCSTRINGS_KEY: &'static str = "docstrings-key";
    const NOTED_TERM: &'static str = "noted-term";

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
            .collect();
        for (term, span) in undefined {
            self.diagnostics
                .push(ExpandProblems::undefined_documented_term(term.name(), span));
        }
    }

    fn skips_document_check(&self, module: ModuleId) -> bool {
        ModuleSources::is_prelude(self.modules.get(module).path())
            || !self.tables.module_notes.holds_any(module)
    }

    fn docstring_notes(&mut self) -> Option<DocstringNotes> {
        let page_module = self.page_module()?;
        let key = self.exported_value(page_module, Self::DOCSTRINGS_KEY)?;
        let noted_term = self.exported_value(page_module, Self::NOTED_TERM)?;
        Some(DocstringNotes {
            key: key.as_form()?.as_symbol()?.shared_name(),
            noted_term,
        })
    }

    fn page_module(&self) -> Option<ModuleId> {
        let prelude = self
            .modules
            .find(&ModulePath::new(ModuleSources::PRELUDE))?;
        let page_macro = self
            .modules
            .get(prelude)
            .exported(Self::PAGE_MACRO, Namespace::Value)?;
        self.definition_module(page_macro)
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
        self.tables
            .module_notes
            .of(module, &notes.key)
            .into_iter()
            .filter_map(|note| self.noted_term(&notes.noted_term, note))
            .collect()
    }

    fn noted_term(&mut self, accessor: &Value, note: Form) -> Option<(Symbol, Span)> {
        let span = note.span();
        let term = Interpreter::new(self, span)
            .call(accessor, vec![Value::Form(note)], span)
            .ok()?;
        let term = term.as_form()?;
        Some((term.as_symbol()?.clone(), term.span()))
    }

    fn defines(&self, module: ModuleId, term: &Symbol) -> bool {
        [Namespace::Value, Namespace::Type]
            .into_iter()
            .filter_map(|namespace| self.try_resolve(term, namespace))
            .any(|binding| self.definition_module(binding) == Some(module))
    }
}
