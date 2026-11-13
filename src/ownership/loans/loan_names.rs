use crate::core::{BindingArena, BindingId};
use crate::source::{SourceMap, Span, TextPosition};

pub struct LoanNames<'n> {
    bindings: &'n BindingArena,
    sources: &'n SourceMap,
}

impl<'n> LoanNames<'n> {
    pub fn new(bindings: &'n BindingArena, sources: &'n SourceMap) -> Self {
        Self { bindings, sources }
    }

    pub fn name(&self, binding: BindingId) -> &'n str {
        self.bindings.get(binding).name()
    }

    pub fn position(&self, span: Span) -> TextPosition {
        self.sources.start_position(span)
    }
}
