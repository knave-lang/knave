use std::collections::BTreeMap;
use std::time::Duration;

use crate::backend::Backend;
use crate::subject_program::SubjectProgram;
use crate::verdict::Verdict;

#[derive(Clone, Debug)]
pub struct BackendVerdicts {
    pub program: SubjectProgram,
    pub verdicts: BTreeMap<&'static str, Verdict>,
}

impl BackendVerdicts {
    pub fn collect(program: SubjectProgram, backends: &[&dyn Backend], limit: Duration) -> Self {
        let verdicts = backends
            .iter()
            .map(|backend| (backend.name(), backend.verdict(&program, limit)))
            .collect();
        Self { program, verdicts }
    }

    pub fn of(&self, backend: &str) -> Option<&Verdict> {
        self.verdicts.get(backend)
    }
}
