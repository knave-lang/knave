use crate::subject_program::SubjectProgram;

pub trait ProgramSource {
    fn programs(&self) -> Box<dyn Iterator<Item = SubjectProgram> + '_>;
}
