use crate::core::BindingArena;
use crate::source::{SourceMap, Span, TextPosition};

pub struct LoanNames<'n> {
    bindings: &'n BindingArena,
    sources: &'n SourceMap,
}

impl<'n> LoanNames<'n> {
    pub fn new(bindings: &'n BindingArena, sources: &'n SourceMap) -> Self {
        Self { bindings, sources }
    }

    pub fn bindings(&self) -> &'n BindingArena {
        self.bindings
    }

    pub fn position(&self, span: Span) -> TextPosition {
        self.sources.start_position(span)
    }
}
