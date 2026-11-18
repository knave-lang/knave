use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) enum ImplementationVerdict {
    Rejected(Diagnostic),
    Accepted(Diagnostics),
}
