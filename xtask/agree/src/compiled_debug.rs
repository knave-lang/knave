use std::time::Duration;

use crate::backend::Backend;
use crate::knave_compiler::KnaveCompiler;
use crate::rustc::Rustc;
use crate::scratch_directory::ScratchDirectory;
use crate::subject_program::SubjectProgram;
use crate::verdict::Verdict;

pub struct CompiledDebug {
    compiler: KnaveCompiler,
}

impl CompiledDebug {
    pub const NAME: &str = "CompiledDebug";

    pub fn new(compiler: KnaveCompiler) -> Self {
        Self { compiler }
    }

    fn emitted_built_and_run(
        &self,
        program: &SubjectProgram,
        directory: &ScratchDirectory,
        limit: Duration,
    ) -> Result<Verdict, Verdict> {
        let emitted = self.compiler.invocation(program, directory).emit(limit)?;
        let binary = Rustc::new(directory.path(), &emitted).build_debug(limit)?;
        Ok(binary.run(program, directory.path(), limit))
    }
}

impl Backend for CompiledDebug {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn verdict(&self, program: &SubjectProgram, limit: Duration) -> Verdict {
        program.judged_in_scratch(|directory| {
            self.emitted_built_and_run(program, directory, limit)
                .unwrap_or_else(|refused| refused)
        })
    }
}
