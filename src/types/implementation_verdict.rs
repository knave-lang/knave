use crate::diagnostic::Diagnostic;

pub(super) enum ImplementationVerdict {
    Rejected(Diagnostic),
    Accepted(Vec<Diagnostic>),
}
