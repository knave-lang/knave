use crate::backend_verdicts::BackendVerdicts;
use crate::subject_program::SubjectProgram;

#[derive(Clone, Debug)]
pub struct Finding {
    pub oracle: &'static str,
    pub program: SubjectProgram,
    pub verdicts: BackendVerdicts,
    pub summary: String,
}

impl Finding {
    pub fn of(oracle: &'static str, verdicts: &BackendVerdicts, summary: String) -> Self {
        Self {
            oracle,
            program: verdicts.program.clone(),
            verdicts: verdicts.clone(),
            summary,
        }
    }
}
