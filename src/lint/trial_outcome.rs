use crate::core::BindingId;
use crate::diagnostic::Diagnostics;

#[derive(Default)]
pub struct TrialOutcome {
    pub warnings: Diagnostics,
    pub removed: Vec<Vec<BindingId>>,
}
