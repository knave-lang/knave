use super::ty::Scheme;
use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) struct SignatureScheme {
    scheme: Scheme,
    unheld: Option<Diagnostic>,
}

impl SignatureScheme {
    pub(super) fn held(scheme: Scheme) -> Self {
        Self {
            scheme,
            unheld: None,
        }
    }

    pub(super) fn with_unheld(scheme: Scheme, unheld: Diagnostic) -> Self {
        Self {
            scheme,
            unheld: Some(unheld),
        }
    }

    pub(super) fn reported_into(self, diagnostics: &mut Diagnostics) -> Scheme {
        if let Some(unheld) = self.unheld {
            diagnostics.push(unheld);
        }
        self.scheme
    }
}
