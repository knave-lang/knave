use crate::diagnostic::Diagnostic;
use crate::reader::{Form, Symbol};
use crate::source::Span;

pub trait DefinitionQueries {
    fn definition_symbol(
        &self,
        term: &Symbol,
        name: String,
        span: Span,
    ) -> Result<Symbol, Diagnostic>;
    fn type_definition_symbol(&self, term: &Symbol, name: String) -> Symbol;
    fn definition_call(&self, term: &Symbol, span: Span) -> Result<Option<Form>, Diagnostic>;
    fn type_definition_call(&self, term: &Symbol) -> Option<Form>;
}
