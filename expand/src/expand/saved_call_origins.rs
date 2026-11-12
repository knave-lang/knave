use super::macro_origin::MacroOrigin;
use super::saved_origins::SavedOrigins;
use crate::source::Span;

#[derive(Default)]
pub struct SavedCallOrigins {
    saved: SavedOrigins,
}

impl SavedCallOrigins {
    pub fn new(saved: SavedOrigins) -> Self {
        Self { saved }
    }

    pub fn at(&self, call_site: Span) -> Option<MacroOrigin> {
        self.saved.origin_at(call_site)
    }

    pub fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        self.saved.enclosing(span)
    }
}
