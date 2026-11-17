use std::rc::Rc;

use crate::source::Span;

pub(super) struct BinderReach {
    pub binder: Span,
    pub introducing_macro: Option<Rc<str>>,
}

impl BinderReach {
    pub fn written_at(binder: Span) -> Self {
        Self {
            binder,
            introducing_macro: None,
        }
    }

    pub fn scope(&self) -> String {
        match &self.introducing_macro {
            Some(name) => format!("in the code of `{name}`"),
            None => "in this scope".to_string(),
        }
    }
}
