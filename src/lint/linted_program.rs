use super::example_runs::ExampleRuns;
use crate::expand::Program;
use crate::types::TypedProgram;

pub struct LintedProgram<'p> {
    pub program: &'p mut Program,
    pub declared: &'p TypedProgram,
    pub examples: ExampleRuns,
    pub holds_unlinted_example: bool,
}
