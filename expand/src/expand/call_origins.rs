use std::rc::Rc;

use super::macro_call_lookup::MacroCallLookup;
use super::macro_origin::MacroOrigin;
use super::own_call_origins::OwnCallOrigins;
use super::saved_call_origins::SavedCallOrigins;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct CallOrigins {
    #[serde(skip)]
    saved: SavedCallOrigins,
    own: OwnCallOrigins,
}

impl CallOrigins {
    pub fn over(saved: SavedCallOrigins) -> Self {
        Self {
            saved,
            own: OwnCallOrigins::default(),
        }
    }

    pub fn record(&mut self, call_site: Span, origin: MacroOrigin) {
        self.own.record(call_site, origin);
    }

    pub fn into_own(self) -> OwnCallOrigins {
        self.own
    }

    #[cfg(test)]
    pub fn holds_no_own_origin(&self) -> bool {
        self.own.origins().is_empty()
    }

    fn call_site_length(origin: &MacroOrigin) -> usize {
        origin.call_site.range().len()
    }
}

impl dyn MacroCallLookup {
    pub fn empty() -> Rc<dyn MacroCallLookup> {
        Rc::new(CallOrigins::default())
    }
}

impl MacroCallLookup for CallOrigins {
    fn introducing_macro(&self, span: Span) -> Option<MacroOrigin> {
        match (self.saved.at(span), self.own.at(span)) {
            (Some(mut saved), Some(own)) => {
                saved.through_prelude |= own.through_prelude;
                Some(saved)
            }
            (saved, own) => saved.or(own),
        }
    }

    fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        let smallest = self
            .saved
            .enclosing(span)
            .into_iter()
            .chain(self.own.enclosing(span))
            .min_by_key(Self::call_site_length)?;
        self.introducing_macro(smallest.call_site)
    }
}
