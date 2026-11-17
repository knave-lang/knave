use crate::reader::Symbol;
use crate::source::{SourceMap, Span};

pub(super) struct BinderSite<'s> {
    symbol: &'s Symbol,
    span: Span,
}

impl<'s> BinderSite<'s> {
    pub fn new(symbol: &'s Symbol, span: Span) -> Self {
        Self { symbol, span }
    }

    pub fn symbol(&self) -> &'s Symbol {
        self.symbol
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn is_spelled_at(&self, sources: &SourceMap, written: Span) -> bool {
        let text = sources.file(written.source()).text();
        text.get(written.range()) == Some(self.symbol.name())
    }
}
