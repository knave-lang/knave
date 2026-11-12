use crate::finding_probe::FindingProbe;
use crate::subject_program::SubjectProgram;

pub trait Reducer: Sync {
    fn shrink(&self, program: SubjectProgram, probe: &FindingProbe) -> SubjectProgram;
}
