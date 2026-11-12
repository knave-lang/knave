use crate::diagnostic::Diagnostic;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadLowering {
    Complete,
    Failed,
}

impl PayloadLowering {
    pub(super) fn of(problems: &[Diagnostic]) -> Self {
        match problems.is_empty() {
            true => Self::Complete,
            false => Self::Failed,
        }
    }
}
