use crate::backend::Backend;
use crate::oracle::Oracle;
use crate::subject_program::SubjectProgram;

pub trait Reducer {
    fn shrink(
        &self,
        program: SubjectProgram,
        oracle: &dyn Oracle,
        backends: &[&dyn Backend],
    ) -> SubjectProgram;
}
