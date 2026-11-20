use crate::diagnostic::Diagnostics;
use crate::expand::Program;

pub struct ProgramWarnings<'p> {
    pub program: &'p Program,
    pub warnings: Diagnostics,
}
