use crate::core::Declaration;
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, Symbol};
use crate::source::Span;

pub trait ExpansionCallbacks {
    fn expand_call(&mut self, call: Form) -> Result<Option<Form>, Diagnostic>;
    fn set_macro_step_limit(&mut self, limit: u64);
    fn declare_definition(&mut self, name: Form, declaration: Declaration);
    fn require_module(&mut self, path: &Symbol, span: Span) -> Result<Symbol, Diagnostic>;
}
