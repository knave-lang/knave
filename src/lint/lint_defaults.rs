use crate::diagnostic::DiagnosticCode;
use crate::project::LintLevel;

pub struct LintDefaults;

impl LintDefaults {
    const ALLOWED_UNLESS_WRITTEN: [DiagnosticCode; 1] = [DiagnosticCode::NEEDLESS_TYPE];

    fn unwritten_level(code: &DiagnosticCode) -> Option<LintLevel> {
        Self::ALLOWED_UNLESS_WRITTEN
            .contains(code)
            .then_some(LintLevel::Allow)
    }

    pub fn level_of(code: &DiagnosticCode, written: Option<LintLevel>) -> Option<LintLevel> {
        written.or_else(|| Self::unwritten_level(code))
    }
}
