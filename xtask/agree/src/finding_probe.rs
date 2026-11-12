use std::time::Duration;

use crate::backend::Backend;
use crate::backend_verdicts::BackendVerdicts;
use crate::finding_signature::FindingSignature;
use crate::oracle::Oracle;
use crate::subject_program::SubjectProgram;

pub struct FindingProbe<'a> {
    oracle: &'a dyn Oracle,
    backends: Vec<&'a dyn Backend>,
    limit: Duration,
}

impl<'a> FindingProbe<'a> {
    pub fn new(oracle: &'a dyn Oracle, backends: Vec<&'a dyn Backend>, limit: Duration) -> Self {
        Self {
            oracle,
            backends,
            limit,
        }
    }

    pub fn signature(&self, program: &SubjectProgram) -> Option<FindingSignature> {
        let verdicts = BackendVerdicts::collect(program.clone(), &self.backends, self.limit);
        self.oracle
            .judge(&verdicts)
            .map(|finding| finding.signature())
    }
}
