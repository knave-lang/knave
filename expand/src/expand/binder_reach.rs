use std::rc::Rc;

use crate::source::Span;

pub(super) enum BinderReach {
    Written {
        binder: Span,
    },
    InMacro {
        binder: Span,
        call: Span,
        introducing_macro: Rc<str>,
    },
}

impl BinderReach {
    pub fn binder(&self) -> Span {
        match self {
            Self::Written { binder } | Self::InMacro { binder, .. } => *binder,
        }
    }
}
