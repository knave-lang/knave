use std::time::Duration;

use crate::backend::Backend;
use crate::knave_compiler::KnaveCompiler;
use crate::rustc::Rustc;
use crate::subject_program::SubjectProgram;
use crate::verdict::Verdict;

pub struct CheckerWithoutOwnership {
    compiler: KnaveCompiler,
}

impl CheckerWithoutOwnership {
    pub const NAME: &str = "CheckerWithoutOwnership";

    pub fn new(compiler: KnaveCompiler) -> Self {
        Self { compiler }
    }
}

impl Backend for CheckerWithoutOwnership {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn verdict(&self, program: &SubjectProgram, limit: Duration) -> Verdict {
        program.judged_in_scratch(|directory| {
            match self
                .compiler
                .invocation(program, directory)
                .emit_waiving_ownership_refusals(limit)
            {
                Ok(emitted) => Rustc::new(directory.path(), &emitted).analyze(limit),
                Err(verdict) => verdict,
            }
        })
    }
}
