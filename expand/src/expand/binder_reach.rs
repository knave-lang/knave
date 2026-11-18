use std::rc::Rc;

use crate::source::Span;

pub(super) enum BinderReach {
    Written {
        binder: Span,
    },
    PassedIn {
        binder: Span,
        call: Span,
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
            Self::Written { binder }
            | Self::PassedIn { binder, .. }
            | Self::InMacro { binder, .. } => *binder,
        }
    }
}
