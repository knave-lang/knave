use crate::project::LintLevel;

#[derive(Clone, Copy)]
pub struct TrialLevels {
    pub needless_type: Option<LintLevel>,
    pub needless_derive: Option<LintLevel>,
}
