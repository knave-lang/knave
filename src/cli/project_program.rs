use super::program_file::ProgramFile;
use crate::expand::Program;
use crate::lint::ExampleRuns;

pub struct ProjectProgram {
    pub file: ProgramFile,
    pub program: Program,
    pub examples: ExampleRuns,
}
