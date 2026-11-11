use crate::diagnostic::Diagnostic;
use crate::reader::Symbol;
use crate::source::Span;

pub trait ModuleQueries {
    fn program_modules(&self) -> Vec<Symbol>;
    fn project_modules(&self) -> Vec<Symbol>;
    fn module_exports(&self, path: &Symbol, span: Span) -> Result<Vec<Symbol>, Diagnostic>;
    fn module_definitions(&self, path: &Symbol, span: Span) -> Result<Vec<Symbol>, Diagnostic>;
    fn current_module(&self) -> Symbol;
    fn module_symbol(&self, path: &Symbol, span: Span) -> Result<Symbol, Diagnostic>;
}
