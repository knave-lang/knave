use std::time::Duration;

use crate::backend::Backend;
use crate::knave_compiler::KnaveCompiler;
use crate::subject_program::SubjectProgram;
use crate::verdict::Verdict;

pub struct Checker {
    compiler: KnaveCompiler,
}

impl Checker {
    pub const NAME: &str = "Checker";

    pub fn new(compiler: KnaveCompiler) -> Self {
        Self { compiler }
    }
}

impl Backend for Checker {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn verdict(&self, program: &SubjectProgram, limit: Duration) -> Verdict {
        program.judged_in_scratch(|directory| {
            match self.compiler.invocation(program, directory).check(limit) {
                Ok(()) => Verdict::Accepted,
                Err(verdict) => verdict,
            }
        })
    }
}
