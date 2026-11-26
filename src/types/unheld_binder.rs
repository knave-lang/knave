use std::rc::Rc;

use crate::core::BindingId;

pub(super) enum UnheldBinder {
    Bare {
        binder: Rc<str>,
    },
    Implementor {
        binder: Rc<str>,
        bound: BindingId,
    },
    InArgument {
        holder: Rc<str>,
        binder: Rc<str>,
        bound: BindingId,
    },
}

impl UnheldBinder {
    pub(super) fn binder(&self) -> &Rc<str> {
        match self {
            Self::Bare { binder }
            | Self::Implementor { binder, .. }
            | Self::InArgument { binder, .. } => binder,
        }
    }

    pub(super) fn bound(&self) -> Option<BindingId> {
        match self {
            Self::Bare { .. } => None,
            Self::Implementor { bound, .. } | Self::InArgument { bound, .. } => Some(*bound),
        }
    }
}
