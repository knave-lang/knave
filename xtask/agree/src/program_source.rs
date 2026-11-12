use std::ops::Range;

use crate::subject_program::SubjectProgram;

pub trait ProgramSource {
    fn programs(&self, seeds: Range<u64>) -> Box<dyn Iterator<Item = SubjectProgram> + '_>;
}
