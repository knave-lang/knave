use std::rc::Rc;

use super::macro_origin::MacroOrigin;
use crate::source::Span;

pub trait MacroCallLookup {
    fn introducing_macro(&self, span: Span) -> Option<MacroOrigin>;
    fn enclosing(&self, span: Span) -> Option<MacroOrigin>;
}

impl dyn MacroCallLookup {
    pub fn empty() -> Rc<dyn MacroCallLookup> {
        Rc::new(NoMacroCalls)
    }
}

struct NoMacroCalls;

impl MacroCallLookup for NoMacroCalls {
    fn introducing_macro(&self, _span: Span) -> Option<MacroOrigin> {
        None
    }

    fn enclosing(&self, _span: Span) -> Option<MacroOrigin> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::MacroCallLookup;
    use crate::expand::origins_fixture::OriginsFixture;

    #[test]
    fn an_empty_lookup_names_no_macro() {
        let lookup = <dyn MacroCallLookup>::empty();
        let call_site = OriginsFixture::call_site();
        assert!(lookup.introducing_macro(call_site).is_none());
        assert!(lookup.enclosing(call_site).is_none());
    }
}
