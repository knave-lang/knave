use std::rc::Rc;

use crate::source::Span;

pub(in crate::types) trait MacroOrigins {
    fn macro_name(&self, span: Span) -> Option<Rc<str>>;
}
