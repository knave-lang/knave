use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::interpret::LimitProblems;

pub struct ExpansionLimit;

impl ExpansionLimit {
    pub fn stopped(failure: &Diagnostic) -> bool {
        Self::ran_too_many_steps(failure)
            || failure.title() == LimitProblems::BUILT_LIST_NESTS_TOO_DEEP
    }

    pub fn ran_too_many_steps(failure: &Diagnostic) -> bool {
        failure.code() == Some(&DiagnosticCode::MACRO_STEP_LIMIT)
    }
}
