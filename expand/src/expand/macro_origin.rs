use std::rc::Rc;

use crate::source::Span;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigin {
    pub name: Rc<str>,
    pub defined: Span,
    pub call_site: Span,
    pub through_prelude: bool,
}
