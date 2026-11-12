use std::time::Duration;

use crate::subject_program::SubjectProgram;
use crate::verdict::Verdict;

pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;
    fn verdict(&self, program: &SubjectProgram, limit: Duration) -> Verdict;
}
