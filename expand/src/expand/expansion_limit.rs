use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::interpret::LimitProblems;

pub struct ExpansionLimit;

impl ExpansionLimit {
    pub fn stopped(failure: &Diagnostic) -> bool {
        failure.code() == Some(&DiagnosticCode::MACRO_STEP_LIMIT)
            || failure.title() == LimitProblems::BUILT_LIST_NESTS_TOO_DEEP
    }
}
