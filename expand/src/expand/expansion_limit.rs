use crate::diagnostic::{Diagnostic, DiagnosticCode};

pub struct ExpansionLimit;

impl ExpansionLimit {
    const CODES: [DiagnosticCode; 1] = [DiagnosticCode::EXPANSION_NESTS_TOO_DEEP];

    pub fn stopped(failure: &Diagnostic) -> bool {
        failure
            .code()
            .is_some_and(|code| Self::CODES.contains(code))
    }
}
