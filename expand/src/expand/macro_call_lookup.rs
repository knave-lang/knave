use super::macro_origin::MacroOrigin;
use crate::source::Span;

pub trait MacroCallLookup {
    fn introducing_macro(&self, span: Span) -> Option<MacroOrigin>;
    fn enclosing(&self, span: Span) -> Option<MacroOrigin>;
}
