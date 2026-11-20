use crate::expand::Program;
use crate::types::TypedProgram;

pub struct CheckedAlone {
    pub program: Program,
    pub declared: TypedProgram,
}
