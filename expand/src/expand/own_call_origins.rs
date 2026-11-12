use std::cell::OnceCell;

use rustc_hash::FxHashMap;

use super::enclosing_index::EnclosingIndex;
use super::macro_origin::MacroOrigin;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct OwnCallOrigins {
    origins: FxHashMap<Span, MacroOrigin>,
    #[serde(skip)]
    enclosing_index: OnceCell<EnclosingIndex>,
}

impl OwnCallOrigins {
    pub fn at(&self, call_site: Span) -> Option<MacroOrigin> {
        self.origins.get(&call_site).cloned()
    }

    pub fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        let index = self
            .enclosing_index
            .get_or_init(|| EnclosingIndex::of(self.origins.keys().copied()));
        self.at(index.smallest_enclosing(span)?)
    }

    pub fn record(&mut self, call_site: Span, origin: MacroOrigin) {
        self.enclosing_index.take();
        self.origins
            .entry(call_site)
            .and_modify(|outer| outer.through_prelude |= origin.through_prelude)
            .or_insert(origin);
    }

    pub fn origins(&self) -> &FxHashMap<Span, MacroOrigin> {
        &self.origins
    }
}
