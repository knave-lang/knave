use crate::expand::Program;
use crate::types::TypedProgram;

pub struct LintedProgram<'p> {
    pub program: &'p mut Program,
    pub declared: &'p TypedProgram,
}
